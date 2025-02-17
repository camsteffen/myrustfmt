use crate::error::{ConstraintError, FormatResult};
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;
use rustc_lexer::Token;
use rustc_lexer::TokenKind;

/// A generic mode of whitespace that knows to respond to the presence of comments
#[derive(Clone, Copy, Debug)]
pub enum WhitespaceMode {
    Horizontal { space: bool },
    Vertical(VerticalWhitespaceMode),
    Flexible {
        space_if_horizontal: bool,
        vertical_mode: VerticalWhitespaceMode,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum VerticalWhitespaceMode {
    /// "between items" where a blank line is allowed. (e.g. between statements)
    Between,
    /// "above" a section, where a blank line is allowed only after comments.
    Above,
    /// "below" a section, where a blank line is allowed only before comments.
    Below,
    /// "within" a construct, where blank lines should be removed.
    Within,
}

#[derive(Debug)]
enum WhitespaceTokenStrategy {
    Horizontal {
        error_on_newline: bool,
        space: bool,
    },
    Vertical { allow_blank_line: bool },
    Flexible {
        space_if_horizontal: bool,
        allow_blank_line: bool,
    },
}

#[derive(Clone, Copy, Debug)]
enum WhitespaceOut {
    Space,
    Newline { double: bool },
}

impl VerticalWhitespaceMode {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            VerticalWhitespaceMode::Between => true,
            VerticalWhitespaceMode::Above => is_comments_before,
            VerticalWhitespaceMode::Below => is_comments_after,
            VerticalWhitespaceMode::Within => is_comments_before && is_comments_after,
        }
    }
}

impl SourceFormatter {
    pub fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        let mut is_required_whitespace_emitted = false;
        let mut seen_comments = false;
        let mut tokens = tokenize_whitespace_and_comments(self.source.remaining())
            .peekable();
        while let Some(token) = tokens.next() {
            let len = token.len as usize;
            let token_str = &self.source.remaining()[..len];
            let has_next = tokens.peek().is_some();
            let result = match token.kind {
                TokenKind::BlockComment { .. }
                | TokenKind::LineComment { .. } => {
                    seen_comments = true;
                    let is_line_comment = matches!(token.kind, TokenKind::LineComment { .. });
                    self.comment_token(mode, len, is_line_comment, has_next)?
                }
                TokenKind::Whitespace => {
                    self.whitespace_token(mode, token_str, seen_comments, has_next)?
                }
                _ => unreachable!(),
            };
            is_required_whitespace_emitted |= result;
        }
        if !is_required_whitespace_emitted {
            self.emit_required_whitespace(mode)?;
        }
        Ok(())
    }

    fn emit_required_whitespace(&self, mode: WhitespaceMode) -> FormatResult {
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
        Ok(())
    }

    fn comment_token(
        &self,
        mode: WhitespaceMode,
        len: usize,
        is_line_comment: bool,
        has_next: bool,
    ) -> FormatResult<bool> {
        let is_horizontal = matches!(mode, WhitespaceMode::Horizontal { .. });
        if is_horizontal && is_line_comment {
            return Err(ConstraintError::NewlineNotAllowed.into());
        }
        self.constraints()
            .max_width
            .with_replaced(None, || self.copy(len))?;
        let mut emitted_required = false;
        if is_horizontal {
            // todo also if vertical and multi-line comment?
            emitted_required = true;
        }
        if is_line_comment && !has_next {
            // end of file newline
            self.out.newline()?;
            emitted_required = true;
        }
        Ok(emitted_required)
    }

    fn whitespace_token(
        &self,
        mode: WhitespaceMode,
        token_str: &str,
        is_comments_before: bool,
        is_comments_after: bool,
    ) -> FormatResult<bool> {
        let out =
            self.determine_whitespace_out(mode, token_str, is_comments_before, is_comments_after)?;
        if let Some(out) = out {
            self.emit_whitespace(out, is_comments_after)?;
        }
        let fulfills_mode = out.is_some_and(|out| whitespace_fulfills_mode(mode, out));
        self.source.advance(token_str.len());
        Ok(fulfills_mode)
    }

    fn determine_whitespace_out(
        &self,
        mode: WhitespaceMode,
        token_str: &str,
        is_comments_before: bool,
        is_comments_after: bool,
    ) -> FormatResult<Option<WhitespaceOut>> {
        let strategy = whitespace_token_strategy(mode, is_comments_before, is_comments_after);
        let out = match strategy {
            WhitespaceTokenStrategy::Horizontal {
                error_on_newline,
                space,
            } => {
                if error_on_newline {
                    if let Some(newline_pos) = token_str.find('\n') {
                        self.source.advance(newline_pos);
                        return Err(ConstraintError::NewlineNotAllowed.into());
                    }
                }
                if !space {
                    return Ok(None);
                }
                WhitespaceOut::Space
            }
            WhitespaceTokenStrategy::Vertical { allow_blank_line } => {
                let double = allow_blank_line && token_str.matches('\n').nth(1).is_some();
                WhitespaceOut::Newline { double }
            }
            WhitespaceTokenStrategy::Flexible {
                allow_blank_line,
                space_if_horizontal,
            } => {
                let mut newlines = token_str.matches('\n');
                if newlines.next().is_some() {
                    let double = allow_blank_line && newlines.next().is_some();
                    WhitespaceOut::Newline { double }
                } else if space_if_horizontal {
                    WhitespaceOut::Space
                } else {
                    return Ok(None);
                }
            }
        };
        Ok(Some(out))
    }

    fn emit_whitespace(&self, whitespace: WhitespaceOut, is_comments_after: bool) -> FormatResult {
        match whitespace {
            WhitespaceOut::Space => self.out.token(" ")?,
            WhitespaceOut::Newline { double } => {
                self.out.newline()?;
                if double {
                    self.out.newline()?;
                }
                if is_comments_after {
                    self.out.indent()?;
                }
            }
        }
        Ok(())
    }
}

fn tokenize_whitespace_and_comments(source: &str) -> impl Iterator<Item = Token> {
    let mut tokens = rustc_lexer::tokenize(source);
    let mut remaining = source;
    std::iter::from_fn(move || {
        let next_char = remaining.chars().next();
        if !next_char.is_some_and(|c| c == '/' || rustc_lexer::is_whitespace(c)) {
            // save the tokenizer some work
            return None;
        }
        let token = tokens.next()?;
        match token.kind {
            TokenKind::BlockComment { .. }
            | TokenKind::LineComment { .. }
            | TokenKind::Whitespace => {
                remaining = &remaining[token.len as usize..];
                Some(token)
            }
            _ => None,
        }
    })
}

fn whitespace_fulfills_mode(mode: WhitespaceMode, out: WhitespaceOut) -> bool {
    match out {
        WhitespaceOut::Space => match mode {
            WhitespaceMode::Horizontal { .. } | WhitespaceMode::Flexible { .. } => true,
            WhitespaceMode::Vertical(_) => false,
        },
        WhitespaceOut::Newline { .. } => true,
    }
}

/// Decides how to handle a whitespace token,
/// knowing whether there are comments before and/or after it
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
