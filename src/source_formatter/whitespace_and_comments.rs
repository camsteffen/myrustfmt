use crate::error::{FormatResult, VerticalError};
use crate::source_formatter::SourceFormatter;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_lexer::{FrontmatterAllowed, Token, TokenKind};

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

    /// Writes a space, allows single-line block comments only.
    pub fn space(&self) -> FormatResult {
        self.whitespace_and_comments(WhitespaceMode::Horizontal { space: true })
    }

    /// Write a space but allow newlines instead if comments are present.
    /// Returns true if newlines were written.
    /// If there are newlines, a trailing newline and indentation is ensured.
    pub fn space_allow_newlines(&self) -> FormatResult<bool> {
        let first_line = self.line();
        self.whitespace_and_comments(WhitespaceMode::Flexible {
            vertical_mode: VerticalWhitespaceMode::Break,
            space_if_horizontal: true,
        })?;
        let has_newlines = self.out.line() > first_line;
        if has_newlines {
            self.indent();
        }
        Ok(has_newlines)
    }

    fn whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        WhitespaceContext {
            sf: self,
            mode,
            is_required_whitespace_emitted: false,
        }
        .whitespace_and_comments()
    }
}

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
    fn vertical_mode(self) -> Option<VerticalWhitespaceMode> {
        match self {
            WhitespaceMode::Horizontal { .. } => None,
            WhitespaceMode::Vertical(mode) => Some(mode),
            WhitespaceMode::Flexible { vertical_mode, .. } => Some(vertical_mode),
        }
    }
}

struct WhitespaceContext<'a> {
    sf: &'a SourceFormatter,
    mode: WhitespaceMode,
    is_required_whitespace_emitted: bool,
}

impl WhitespaceContext<'_> {
    fn whitespace_and_comments<'a>(&mut self) -> FormatResult {
        let Self { sf, mode, .. } = *self;

        let mut remaining_source = sf.source_reader.remaining();
        let tokens = tokenize_whitespace_and_comments(remaining_source);
        let mut tokens = tokens.peekable();

        let mut seen_comments = false;
        let mut last_is_line_comment = false;

        while let Some(token) = tokens.next() {
            let token_str;
            (token_str, remaining_source) = remaining_source
                .split_at(token.len.try_into().unwrap());
            let (is_comment, is_line_comment) = match token.kind {
                TokenKind::LineComment { .. } => (true, true),
                TokenKind::BlockComment { .. } => (true, false),
                TokenKind::Whitespace => (false, false),
                _ => unreachable!(),
            };
            if is_comment {
                if matches!(mode, WhitespaceMode::Horizontal { .. }) {
                    if is_line_comment {
                        return Err(VerticalError::LineComment.into());
                    } else if token_str.contains('\n') {
                        return Err(VerticalError::MultiLineComment.into());
                    }
                }
                if let Some((i, _)) = token_str
                    .char_indices()
                    .rev()
                    .take_while(|(_, c)| c.is_whitespace())
                    .last()
                {
                    self.copy_comment(i as u32)?;
                    // skip trailing whitespace
                    self.advance_source(token.len - i as u32);
                } else {
                    self.copy_comment(token.len)?;
                }
            } else {
                let has_comments_after = tokens.peek().is_some();
                let is_newline =
                    self.whitespace_token(token_str, token.len, seen_comments, has_comments_after)?;
                if mode.vertical_mode() == Some(VerticalWhitespaceMode::SingleNewline) && is_newline
                {
                    break;
                }
            }
            seen_comments |= is_comment;
            last_is_line_comment = is_line_comment;
        }
        if last_is_line_comment {
            // add a trailing newline
            self.emit_newline(0, false, false)?;
        } else if !self.is_required_whitespace_emitted {
            match self.mode {
                WhitespaceMode::Horizontal { space }
                | WhitespaceMode::Flexible {
                    space_if_horizontal: space,
                    ..
                } => {
                    if space {
                        sf.out.token(" ")?;
                    }
                }
                WhitespaceMode::Vertical(_) => sf.out.newline()?,
            }
        }
        Ok(())
    }

    fn whitespace_token(
        &mut self,
        token_str: &str,
        token_len: u32,
        has_comments_before: bool,
        has_comments_after: bool,
    ) -> FormatResult<bool> {
        let mut is_newline = false;
        match whitespace_token_strategy(self.mode, has_comments_before, has_comments_after) {
            WhitespaceTokenStrategy::Horizontal {
                error_on_newline,
                space,
            } => {
                if error_on_newline && let Some(newline_pos) = token_str.find('\n') {
                    if newline_pos > 0 {
                        // todo add a test - probably necessary for accurate error output
                        self.advance_source(newline_pos as u32);
                    }
                    return Err(VerticalError::Newline.into());
                }
                if !space {
                    self.advance_source(token_len);
                    return Ok(is_newline);
                }
                self.emit_space(token_len)?;
            }
            WhitespaceTokenStrategy::Vertical { allow_blank_line } => {
                let double = allow_blank_line && token_str.matches('\n').nth(1).is_some();
                self.emit_newline(token_len, double, has_comments_after)?;
                is_newline = true;
            }
            WhitespaceTokenStrategy::Flexible {
                allow_blank_line,
                space_if_horizontal,
            } => {
                let mut newlines = token_str.matches('\n');
                if newlines.next().is_some() {
                    let double = allow_blank_line && newlines.next().is_some();
                    self.emit_newline(token_len, double, has_comments_after)?;
                    is_newline = true;
                } else if space_if_horizontal {
                    self.emit_space(token_len)?;
                } else {
                    self.advance_source(token_len);
                }
            }
        }
        Ok(is_newline)
    }
}

// lower-level functions
impl WhitespaceContext<'_> {
    fn advance_source(&self, input_len: u32) {
        self.sf.source_reader.advance(input_len)
    }

    fn copy_comment(&self, len: u32) -> FormatResult {
        // width limits don't apply to comments
        self.sf
            .constraints()
            .with_replace_width_limit(None, || self.sf.copy(len))
    }

    fn emit_newline(&mut self, input_len: u32, double: bool, indent: bool) -> FormatResult {
        self.sf.out.newline()?;
        if double {
            self.sf.out.newline()?;
        }
        if indent {
            self.sf.indent();
        }
        self.is_required_whitespace_emitted = true;
        self.advance_source(input_len);
        Ok(())
    }

    fn emit_space(&mut self, input_len: u32) -> FormatResult {
        self.sf.out.token(" ")?;
        match self.mode {
            WhitespaceMode::Horizontal { .. } | WhitespaceMode::Flexible { .. } => {
                self.is_required_whitespace_emitted = true;
            }
            WhitespaceMode::Vertical(_) => {}
        }
        self.advance_source(input_len);
        Ok(())
    }
}

fn tokenize_whitespace_and_comments(source: &str) -> impl Iterator<Item = Token> {
    let mut cursor = rustc_lexer::Cursor::new(source, FrontmatterAllowed::No);
    std::iter::from_fn(move || {
        let remaining = cursor.as_str();
        let next_char = remaining.chars().next();
        if !next_char.is_some_and(|c| c == '/' || rustc_lexer::is_whitespace(c)) {
            // optimization: whatever comes next isn't whitespace or comments, so don't parse it
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

/// Describes how to handle a whitespace token without yet knowing its contents
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

fn whitespace_token_strategy(
    mode: WhitespaceMode,
    has_comments_before: bool,
    has_comments_after: bool,
) -> WhitespaceTokenStrategy {
    let is_by_comments = has_comments_before || has_comments_after;
    match mode {
        WhitespaceMode::Horizontal { space } => WhitespaceTokenStrategy::Horizontal {
            error_on_newline: has_comments_before,
            space: space || is_by_comments,
        },
        WhitespaceMode::Vertical(mode) => {
            let allow_blank_line = mode.allow_blank_line(has_comments_before, has_comments_after);
            if has_comments_after {
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
                        .allow_blank_line(has_comments_before, has_comments_after),
                    space_if_horizontal: true,
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
