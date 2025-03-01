use crate::error::{ConstraintError, FormatResult};
use crate::source_formatter::SourceFormatter;
use rustc_lexer::Token;
use rustc_lexer::TokenKind;

/// Wherever there is whitespace, and possibly comments, a WhitespaceMode is used to format it
#[derive(Clone, Copy, Debug)]
pub enum WhitespaceMode {
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

#[derive(Clone, Copy, Debug)]
pub enum VerticalWhitespaceMode {
    /// "between items" where a blank line is allowed. (e.g. between statements or items)
    Between,
    /// at the top of a file or block - a blank line is allowed only after comments
    Top,
    /// at the bottom of a file or block - a blank line is allowed only before comments.
    Bottom,
    /// a line break where blank lines should be removed, usually breaking a construct into
    /// multiple lines that could have been on one line
    Break,
}

#[derive(Debug)]
enum WhitespaceTokenStrategy {
    /// Coerce into horizontal space
    Horizontal {
        error_on_newline: bool,
        space: bool,
    },
    /// Coerce into vertical space
    Vertical {
        allow_blank_line: bool,
    },
    /// Emit horizontal space by default, but preserve newlines by comments
    Flexible {
        space_if_horizontal: bool,
        allow_blank_line: bool,
    },
}

enum WhitespaceOutput {
    Nothing,
    CopyComment,
    EmitNewline { double: bool, indent: bool },
    EmitSpace,
    LineCommentNotAllowed,
    NewlineNotAllowed { distance: usize },
}

impl VerticalWhitespaceMode {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            VerticalWhitespaceMode::Between => true,
            VerticalWhitespaceMode::Top => is_comments_before,
            VerticalWhitespaceMode::Bottom => is_comments_after,
            VerticalWhitespaceMode::Break => is_comments_before && is_comments_after,
        }
    }
}

impl SourceFormatter {
    pub fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        let mut is_required_whitespace_emitted = false;
        let tokens = tokenize(self.source.remaining());
        let outputs = outputs_from_tokens(tokens, mode);
        for (output, len) in outputs {
            match output {
                WhitespaceOutput::CopyComment => {
                    self
                        .constraints()
                        .with_max_width(None, || self.copy(len as usize))?;
                },
                WhitespaceOutput::EmitNewline { double, indent } => {
                    self.source.advance(len as usize);
                    self.out.newline()?;
                    if double {
                        self.out.newline()?;
                    }
                    if indent {
                        self.indent();
                    }
                    is_required_whitespace_emitted = true;
                }
                WhitespaceOutput::EmitSpace => {
                    self.source.advance(len as usize);
                    self.out.token(" ")?;
                    match mode {
                        WhitespaceMode::Horizontal { .. } | WhitespaceMode::Flexible { .. } => {
                            is_required_whitespace_emitted = true;
                        }
                        WhitespaceMode::Vertical(_) => {}
                    }
                }
                WhitespaceOutput::Nothing => {
                    self.source.advance(len as usize);
                }
                WhitespaceOutput::NewlineNotAllowed { distance } => {
                    self.source.advance(distance);
                    return Err(ConstraintError::NewlineNotAllowed.into());
                }
                WhitespaceOutput::LineCommentNotAllowed => {
                    return Err(ConstraintError::NewlineNotAllowed.into());
                }
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
fn tokenize(source: &str) -> impl Iterator<Item = (Token, &str)> {
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
            | TokenKind::Whitespace => {
                let token_str = &remaining[..token.len as usize];
                Some((token, token_str))
            }
            _ => None,
        }
    })
}

/// Iterate over tokens and decide what to output
fn outputs_from_tokens<'a, 'b>(
    tokens: impl Iterator<Item = (Token, &'a str)> + 'b,
    mode: WhitespaceMode,
) -> impl Iterator<Item = (WhitespaceOutput, u32)> + 'b {
    let mut tokens = tokens.peekable();
    let mut seen_comments = false;
    let mut last_is_line_comment = false;
    std::iter::from_fn(move || {
        let Some((token, token_str)) = tokens.next() else {
            if last_is_line_comment {
                last_is_line_comment = false;
                // add trailing newline
                return Some((WhitespaceOutput::EmitNewline {double: false, indent: false}, 0));
            }
            return None
        };
        let (is_comment, is_line_comment) = match token.kind {
            TokenKind::LineComment { .. } => (true, true),
            TokenKind::BlockComment { .. } => (true, false),
            TokenKind::Whitespace => (false, false),
            _ => unreachable!(),
        };
        let action = if mode.is_horizontal() && is_line_comment {
            WhitespaceOutput::LineCommentNotAllowed
        } else if is_comment {
            WhitespaceOutput::CopyComment
        } else {
            let is_comments_after = tokens.peek().is_some();
            let strategy = whitespace_token_strategy(mode, seen_comments, is_comments_after);
            whitespace_token_action(token_str, strategy, is_comments_after)
        };
        seen_comments |= is_comment;
        last_is_line_comment = is_line_comment;
        Some((action, token.len))
    })
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
    strategy: WhitespaceTokenStrategy,
    is_comments_after: bool,
) -> WhitespaceOutput {
    match strategy {
        WhitespaceTokenStrategy::Horizontal {
            error_on_newline,
            space,
        } => {
            if error_on_newline {
                if let Some(newline_pos) = token_str.find('\n') {
                    return WhitespaceOutput::NewlineNotAllowed {
                        distance: newline_pos,
                    };
                }
            }
            if !space {
                return WhitespaceOutput::Nothing;
            }
            WhitespaceOutput::EmitSpace
        }
        WhitespaceTokenStrategy::Vertical { allow_blank_line } => {
            let double = allow_blank_line && token_str.matches('\n').nth(1).is_some();
            WhitespaceOutput::EmitNewline {
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
                WhitespaceOutput::EmitNewline {
                    double,
                    indent: is_comments_after,
                }
            } else if space_if_horizontal {
                WhitespaceOutput::EmitSpace
            } else {
                WhitespaceOutput::Nothing
            }
        }
    }
}
