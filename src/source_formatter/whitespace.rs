use crate::error::{ConstraintErrorKind, FormatResult};
use crate::source_formatter::SourceFormatter;
use rustc_lexer::Token;
use rustc_lexer::TokenKind;
use crate::whitespace::VerticalWhitespaceMode;

impl SourceFormatter {
    /// Allows any comments or nothing.
    /// This is not very commonly used because comments come "free" with newline and space functions
    /// (although spaces only allow single-line comments).
    pub fn comments(&self, mode: VerticalWhitespaceMode) -> FormatResult {
        self.whitespace_and_comments(WhitespaceMode::Flexible {
            vertical_mode: mode,
            space_if_horizontal: false,
        })
    }

    /// Try to write a space but also allow a line break with some comments
    pub fn space_or_break(&self) -> FormatResult {
        let first_line = self.line();
        self.whitespace_and_comments(WhitespaceMode::Flexible {
            vertical_mode: VerticalWhitespaceMode::Break,
            space_if_horizontal: true,
        })?;
        if self.out.line() != first_line {
            self.indent();
        }
        Ok(())
    }

    /// Skip over whitespace, allow horizontal comments, disallow newlines.
    /// In other words, usually do nothing but allow for comments.
    /// SourceFormatter is responsible for invoking this between tokens.
    // todo maybe AstFormatter should call this
    pub(super) fn horizontal_whitespace(&self) -> FormatResult {
        self.whitespace_and_comments(WhitespaceMode::Horizontal { space: false })
    }

    pub fn indent(&self) {
        self.out.spaces(self.total_indent.get());
    }

    /// Write a newline, allow comments
    pub fn newline(&self, mode: VerticalWhitespaceMode) -> FormatResult {
        self.whitespace_and_comments(WhitespaceMode::Vertical(mode))
    }

    pub fn newline_indent(&self, mode: VerticalWhitespaceMode) -> FormatResult {
        self.newline(mode)?;
        self.indent();
        Ok(())
    }

    /// Write a space, allow horizontal comments
    pub fn space(&self) -> FormatResult {
        self.whitespace_and_comments(WhitespaceMode::Horizontal { space: true })
    }
}

/// Wherever there is whitespace, and possibly comments, a WhitespaceMode is used to format it
#[derive(Clone, Copy, Debug)]
enum WhitespaceMode {
    /// Horizontal space only. Error on newlines or line comments.
    Horizontal { space: bool },
    /// One or more newlines
    Vertical(VerticalWhitespaceMode),
    /// Horizontal by default but can be vertical when there are comments
    Flexible {
        space_if_horizontal: bool,
        vertical_mode: VerticalWhitespaceMode,
    },
}

impl WhitespaceMode {
    pub fn is_horizontal(self) -> bool {
        matches!(self, WhitespaceMode::Horizontal { .. })
    }
}

impl SourceFormatter {
    fn whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        let mut is_required_whitespace_emitted = false;
        let source = self.source_reader.remaining();
        let tokens = tokenize(source);
        let actions = actions_from_tokens(tokens, mode, source);
        for (action, len) in actions {
            match action {
                WhitespaceAction::CopyComment => {
                    // width limits don't apply to comments
                    self.constraints()
                        .with_replace_width_limit(None, || self.copy(len))?;
                }
                WhitespaceAction::EmitNewline { double, indent } => {
                    self.source_reader.advance(len);
                    self.out.newline()?;
                    if double {
                        self.out.newline()?;
                    }
                    if indent {
                        self.indent();
                    }
                    is_required_whitespace_emitted = true;
                }
                WhitespaceAction::EmitSpace => {
                    self.source_reader.advance(len);
                    self.out.token(" ")?;
                    match mode {
                        WhitespaceMode::Horizontal { .. } | WhitespaceMode::Flexible { .. } => {
                            is_required_whitespace_emitted = true;
                        }
                        WhitespaceMode::Vertical(_) => {}
                    }
                }
                WhitespaceAction::NewlineNotAllowed { distance } => {
                    self.source_reader.advance(distance);
                    return Err(ConstraintErrorKind::NewlineNotAllowed.into());
                }
                WhitespaceAction::LineCommentNotAllowed => {
                    return Err(ConstraintErrorKind::LineCommentNotAllowed.into());
                }
                WhitespaceAction::MultiLineCommentNotAllowed => {
                    return Err(ConstraintErrorKind::MultiLineCommentNotAllowed.into());
                }
                WhitespaceAction::Skip => self.source_reader.advance(len),
            }
        }
        if !is_required_whitespace_emitted {
            match mode {
                WhitespaceMode::Horizontal { space }
                | WhitespaceMode::Flexible {
                    space_if_horizontal: space,
                    ..
                } => {
                    if space {
                        self.out.token(" ")?;
                    }
                }
                WhitespaceMode::Vertical(_) => self.out.newline()?,
            }
        }
        Ok(())
    }
}

/// Tokenize whitespace and comment tokens. Stop upon encountering anything else.
fn tokenize(source: &str) -> impl Iterator<Item = Token> {
    let mut cursor = rustc_lexer::Cursor::new(source);
    std::iter::from_fn(move || {
        let remaining = cursor.as_str();
        let next_char = remaining.chars().next();
        if !next_char.is_some_and(|c| c == '/' || rustc_lexer::is_whitespace(c)) {
            // optimization: don't parse some token we don't care about
            return None;
        }
        let token = cursor.advance_token();
        match token.kind {
            TokenKind::BlockComment { .. }
            | TokenKind::LineComment { .. }
            | TokenKind::Whitespace => Some(token),
            _ => None,
        }
    })
}

enum WhitespaceAction {
    CopyComment,
    EmitNewline { double: bool, indent: bool },
    EmitSpace,
    LineCommentNotAllowed,
    MultiLineCommentNotAllowed,
    NewlineNotAllowed { distance: u32 },
    Skip,
}

/// Iterate over tokens and decide what to output
fn actions_from_tokens<'a>(
    tokens: impl Iterator<Item = Token> + 'a,
    mode: WhitespaceMode,
    source: &'a str,
) -> impl Iterator<Item = (WhitespaceAction, u32)> + 'a {
    let mut tokens = tokens.peekable();
    let mut seen_comments = false;
    let mut last_is_line_comment = false;
    let mut remaining = source;
    std::iter::from_fn(move || {
        let Some(token) = tokens.next() else {
            if last_is_line_comment {
                last_is_line_comment = false;
                // add trailing newline
                return Some((
                    WhitespaceAction::EmitNewline {
                        double: false,
                        indent: false,
                    },
                    0,
                ));
            }
            return None;
        };
        let token_str;
        (token_str, remaining) = remaining.split_at(token.len.try_into().unwrap());
        let (is_comment, is_line_comment) = match token.kind {
            TokenKind::LineComment { .. } => (true, true),
            TokenKind::BlockComment { .. } => (true, false),
            TokenKind::Whitespace => (false, false),
            _ => unreachable!(),
        };
        let action = if is_comment {
            if mode.is_horizontal() {
                if is_line_comment {
                    WhitespaceAction::LineCommentNotAllowed
                } else if token_str.contains('\n') {
                    WhitespaceAction::MultiLineCommentNotAllowed
                } else {
                    WhitespaceAction::CopyComment
                }
            } else {
                WhitespaceAction::CopyComment
            }
        } else {
            let is_comments_after = tokens.peek().is_some();
            let strategy = whitespace_token_strategy(mode, seen_comments, is_comments_after);
            whitespace_token_action(token_str, token.len, strategy, is_comments_after)
        };
        seen_comments |= is_comment;
        last_is_line_comment = is_line_comment;
        Some((action, token.len))
    })
}

#[derive(Debug)]
enum WhitespaceTokenStrategy {
    /// Coerce into horizontal space
    Horizontal { error_on_newline: bool, space: bool },
    /// Coerce into vertical space
    Vertical { allow_blank_line: bool },
    /// Emit horizontal space by default, but preserve newlines by comments
    Flexible {
        space_if_horizontal: bool,
        allow_blank_line: bool,
    },
}

/// Computes the strategy for handling a whitespace token based on the mode and the presence of
/// surrounding comments
fn whitespace_token_strategy(
    mode: WhitespaceMode,
    is_comments_before: bool,
    is_comments_after: bool,
) -> WhitespaceTokenStrategy {
    let is_by_comments = is_comments_before || is_comments_after;
    match mode {
        WhitespaceMode::Horizontal { space } => WhitespaceTokenStrategy::Horizontal {
            error_on_newline: is_comments_before,
            space: space || is_by_comments,
        },
        WhitespaceMode::Vertical(mode) => {
            let allow_blank_line = mode.allow_blank_line(is_comments_before, is_comments_after);
            if is_comments_after {
                WhitespaceTokenStrategy::Flexible {
                    allow_blank_line,
                    space_if_horizontal: true,
                }
            } else {
                WhitespaceTokenStrategy::Vertical { allow_blank_line }
            }
        }
        WhitespaceMode::Flexible {
            space_if_horizontal,
            vertical_mode,
        } => {
            if is_by_comments {
                WhitespaceTokenStrategy::Flexible {
                    allow_blank_line: vertical_mode
                        .allow_blank_line(is_comments_before, is_comments_after),
                    space_if_horizontal: space_if_horizontal || is_by_comments,
                }
            } else {
                WhitespaceTokenStrategy::Horizontal {
                    error_on_newline: false,
                    space: space_if_horizontal,
                }
            }
        }
    }
}

/// Finally decides what to do with a whitespace token
fn whitespace_token_action(
    token_str: &str,
    token_len: u32,
    strategy: WhitespaceTokenStrategy,
    is_comments_after: bool,
) -> WhitespaceAction {
    match strategy {
        WhitespaceTokenStrategy::Horizontal {
            error_on_newline,
            space,
        } => {
            if error_on_newline {
                if let Some(newline_pos) = token_str.find('\n') {
                    // since token_len is u32, newline_pos is bound by u32
                    let _: u32 = token_len;
                    let distance = u32::try_from(newline_pos).unwrap();
                    return WhitespaceAction::NewlineNotAllowed { distance };
                }
            }
            if !space {
                return WhitespaceAction::Skip;
            }
            WhitespaceAction::EmitSpace
        }
        WhitespaceTokenStrategy::Vertical { allow_blank_line } => {
            let double = allow_blank_line && token_str.matches('\n').nth(1).is_some();
            WhitespaceAction::EmitNewline {
                double,
                indent: is_comments_after,
            }
        }
        WhitespaceTokenStrategy::Flexible {
            allow_blank_line,
            space_if_horizontal,
        } => {
            let mut newlines = token_str.matches('\n');
            if newlines.next().is_some() {
                let double = allow_blank_line && newlines.next().is_some();
                WhitespaceAction::EmitNewline {
                    double,
                    indent: is_comments_after,
                }
            } else if space_if_horizontal {
                WhitespaceAction::EmitSpace
            } else {
                WhitespaceAction::Skip
            }
        }
    }
}
