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
    VerticalIfComments {
        space_if_horizontal: bool,
        vertical_mode: VerticalWhitespaceMode,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum VerticalWhitespaceMode {
    /// Newline between items where a blank line is allowed. (e.g. between statements)
    Between,
    /// Newline at the beginning of a section, where a blank line is allowed only after comments.
    Above,
    /// Newline at the end of a section, where a blank line is allowed only before comments.
    Below,
    /// Newline in a place where blank lines should be removed.
    Within,
}

#[derive(Debug)]
enum WhitespaceTokenStrategy {
    Horizontal {
        error_if_newlines: bool,
        space: bool,
    },
    VerticalIfNewlines {
        space_if_horizontal: bool,
        allow_blank_line: bool,
    },
    Vertical { allow_blank_line: bool },
}

impl WhitespaceMode {
    fn whitespace_token_strategy(
        self,
        is_comments_before: bool,
        is_comments_after: bool,
    ) -> WhitespaceTokenStrategy {
        let is_by_comments = is_comments_before || is_comments_after;
        match self {
            WhitespaceMode::Horizontal { space } => WhitespaceTokenStrategy::Horizontal {
                error_if_newlines: is_comments_before,
                space: space || is_by_comments,
            },
            WhitespaceMode::Vertical(mode) => {
                let allow_blank_line = mode.allow_blank_line(is_comments_before, is_comments_after);
                if is_comments_after {
                    WhitespaceTokenStrategy::VerticalIfNewlines {
                        allow_blank_line,
                        space_if_horizontal: true,
                    }
                } else {
                    WhitespaceTokenStrategy::Vertical { allow_blank_line }
                }
            }
            WhitespaceMode::VerticalIfComments {
                space_if_horizontal,
                vertical_mode,
            } => {
                if is_by_comments {
                    WhitespaceTokenStrategy::VerticalIfNewlines {
                        allow_blank_line: vertical_mode
                            .allow_blank_line(is_comments_before, is_comments_after),
                        space_if_horizontal: space_if_horizontal || is_by_comments,
                    }
                } else {
                    WhitespaceTokenStrategy::Horizontal {
                        error_if_newlines: false,
                        space: space_if_horizontal,
                    }
                }
            }
        }
    }
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

struct WhitespaceContext {
    is_comments_before: bool,
    is_required_whitespace_emitted: bool,
    mode: WhitespaceMode,
}

impl SourceFormatter {
    // todo optimize no-op case?
    pub fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        let is_horizontal = matches!(mode, WhitespaceMode::Horizontal { .. });
        let wcx = &mut WhitespaceContext {
            is_comments_before: false,
            is_required_whitespace_emitted: false,
            mode,
        };
        let mut tokens = tokenize_whitespace_and_comments(self.source.remaining())
            .peekable();
        while let Some(token) = tokens.next() {
            match token.kind {
                TokenKind::LineComment { .. } if is_horizontal => {
                    return Err(ConstraintError::NewlineNotAllowed.into());
                }
                TokenKind::BlockComment { .. }
                | TokenKind::LineComment { .. } => {
                    self.constraints()
                        .max_width
                        .with_replaced(None, || self.copy(token.len as usize))?;
                    wcx.is_comments_before = true;
                    if is_horizontal {
                        // todo also if vertical and multi-line comment?
                        wcx.is_required_whitespace_emitted = true;
                    }
                    if matches!(token.kind, TokenKind::LineComment { .. })
                        && tokens.peek().is_none()
                    {
                        // end of file newline
                        self.out.newline()?;
                        wcx.is_required_whitespace_emitted = true;
                    }
                }
                TokenKind::Whitespace => {
                    let token_str = &self.source.remaining()[..token.len as usize];
                    let is_comments_after = tokens.peek().is_some();
                    self.flush_whitespace(wcx, token_str, is_comments_after)?;
                    self.source.advance(token_str.len());
                }
                _ => unreachable!(),
            }
        }
        self.ensure_required_whitespace(wcx)?;

        Ok(())
    }

    fn ensure_required_whitespace(&self, wcx: &mut WhitespaceContext) -> FormatResult {
        if !wcx.is_required_whitespace_emitted {
            match wcx.mode {
                WhitespaceMode::Horizontal { space }
                | WhitespaceMode::VerticalIfComments {
                    space_if_horizontal: space,
                    ..
                } => {
                    if space {
                        self.out.token(" ")?;
                    }
                }
                WhitespaceMode::Vertical(_) => self.out.newline()?,
            }
            wcx.is_required_whitespace_emitted = true;
        }
        Ok(())
    }

    fn flush_whitespace(
        &self,
        wcx: &mut WhitespaceContext,
        token_str: &str,
        is_comments_after: bool,
    ) -> FormatResult {
        let strategy =
            wcx.mode.whitespace_token_strategy(wcx.is_comments_before, is_comments_after);
        enum Emit {
            Space,
            Newline { double: bool },
        }
        let emit = match strategy {
            WhitespaceTokenStrategy::Horizontal {
                error_if_newlines,
                space,
            } => {
                if error_if_newlines {
                    if let Some(newline_pos) = token_str.find('\n') {
                        self.source.advance(newline_pos);
                        return Err(ConstraintError::NewlineNotAllowed.into());
                    }
                }
                if !space {
                    return Ok(())
                }
                Emit::Space
            }
            WhitespaceTokenStrategy::Vertical { allow_blank_line } => {
                let double = allow_blank_line && token_str.matches('\n').nth(1).is_some();
                Emit::Newline { double }
            }
            WhitespaceTokenStrategy::VerticalIfNewlines {
                allow_blank_line,
                space_if_horizontal,
            } => {
                let mut newlines = token_str.matches('\n');
                if newlines.next().is_some() {
                    let double = allow_blank_line && newlines.next().is_some();
                    Emit::Newline { double }
                } else if space_if_horizontal {
                    Emit::Space
                } else {
                    return Ok(())
                }
            }
        };
        match emit {
            Emit::Space => {
                self.out.token(" ")?;
                match wcx.mode {
                    WhitespaceMode::Horizontal { .. }
                    | WhitespaceMode::VerticalIfComments { .. } => {
                        wcx.is_required_whitespace_emitted = true;
                    }
                    WhitespaceMode::Vertical(_) => {}
                }
            }
            Emit::Newline { double } => {
                self.out.newline()?;
                if double {
                    self.out.newline()?;
                }
                if is_comments_after {
                    self.out.indent()?;
                }
                wcx.is_required_whitespace_emitted = true;
            }
        }
        Ok(())
    }
}

fn tokenize_whitespace_and_comments(source: &str) -> impl Iterator<Item = Token> {
    let mut tokens = rustc_lexer::tokenize(source);
    let mut remaining = source;
    std::iter::from_fn(move || {
        let next_char = remaining.chars().next()?;
        if !(next_char == '/' || rustc_lexer::is_whitespace(next_char)) {
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
