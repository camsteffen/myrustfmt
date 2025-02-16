use crate::error::{ConstraintError, FormatResult};
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;
use rustc_lexer::Token;
use rustc_lexer::TokenKind;

/// What whitespace do we want to print, and how to respond to comments
#[derive(Clone, Copy)]
pub enum WhitespaceMode {
    Horizontal {
        /// Whether to simply stop with Ok or return an error
        // todo dis always true??
        error_on_newline: bool,
        /// True if we want to print a space, otherwise we're just allowing for comments
        space: bool,
    },
    Vertical { kind: NewlineKind, min: NewlineMin },
}

#[derive(Clone, Copy, Debug)]
pub enum NewlineMin {
    Zero,
    One,
    // todo force blank lines between items?
    // Two,
}

#[derive(Clone, Copy, Debug)]
pub enum NewlineKind {
    /// Newline between items where a blank line is allowed. (e.g. between statements)
    Between,
    /// Newline at the beginning of a braced section. A blank line is allowed only below comments.
    Above,
    /// Newline at the end of a braced section. A blank line is allowed only above comments.
    Below,
    /// Newline in a place where extra blank lines should be trimmed away.
    Within,
}

impl NewlineKind {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            NewlineKind::Between => true,
            NewlineKind::Above => is_comments_before,
            NewlineKind::Below => is_comments_after,
            NewlineKind::Within => is_comments_before && is_comments_after,
        }
    }
}

struct WhitespaceContext {
    is_comments_before: bool,
    /// In horizontal mode, true if any whitespace or comments are emitted.
    /// In vertical mode, true if any newlines emitted.
    is_whitespace_mode_out: bool,
    mode: WhitespaceMode,
}

impl SourceFormatter {
    // todo optimize no-op case?
    pub fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult<bool> {
        let wcx = &mut WhitespaceContext {
            is_comments_before: false,
            is_whitespace_mode_out: false,
            mode,
        };
        let mut tokens = tokenize_whitespace_and_comments(self.source.remaining())
            .peekable();
        while let Some(token) = tokens.next() {
            match token.kind {
                TokenKind::LineComment { .. }
                    if matches!(
                        wcx.mode,
                        WhitespaceMode::Horizontal {
                            error_on_newline: true,
                            ..
                        }
                    ) =>
                {
                    return Err(ConstraintError::NewlineNotAllowed.into());
                }
                TokenKind::BlockComment { .. }
                | TokenKind::LineComment { .. } => {
                    self.constraints()
                        .max_width
                        .with_replaced(None, || self.copy(token.len as usize))?;
                    wcx.is_comments_before = true;
                    if matches!(wcx.mode, WhitespaceMode::Horizontal { .. }) {
                        wcx.is_whitespace_mode_out = true;
                    }
                    if matches!(token.kind, TokenKind::LineComment { .. })
                        && tokens.peek().is_none()
                    {
                        // end of file newline
                        self.out.newline()?;
                    }
                }
                TokenKind::Whitespace => {
                    let mut take_whitespace = |wcx, len, newlines| -> FormatResult {
                        let is_comments_after = tokens.peek().is_some_and(|token| matches!(
                                token.kind,
                                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. }
                            ));
                        self.flush_whitespace(wcx, len, newlines, is_comments_after)?;
                        Ok(())
                    };
                    let token_str = &self.source.remaining()[..token.len as usize];
                    match wcx.mode {
                        WhitespaceMode::Horizontal {
                            error_on_newline,
                            ..
                        }
                            if wcx.is_comments_before =>
                        {
                            // todo avoid tokenizing the whole whitespace token?
                            match token_str.find('\n') {
                                None => take_whitespace(wcx, token.len as usize, 0)?,
                                Some(newline_pos) => {
                                    if error_on_newline {
                                        // todo eat whitespace before the newline?
                                        return Err(ConstraintError::NewlineNotAllowed.into());
                                    }
                                    if newline_pos > 0 {
                                        take_whitespace(wcx, newline_pos, 0)?;
                                    }
                                    break;
                                }
                            }
                        }
                        _ => {
                            let newlines = token_str.bytes().filter(|&b| b == b'\n').count();
                            take_whitespace(wcx, token.len as usize, newlines)?;
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        self.ensure_required_whitespace(wcx)?;

        Ok(wcx.is_whitespace_mode_out)
    }

    fn ensure_required_whitespace(&self, wcx: &mut WhitespaceContext) -> FormatResult {
        if wcx.is_whitespace_mode_out {
            return Ok(());
        }
        match wcx.mode {
            WhitespaceMode::Horizontal {
                error_on_newline: _,
                space,
            } => {
                if space {
                    self.out.token(" ")?;
                    wcx.is_whitespace_mode_out = true;
                }
            }
            WhitespaceMode::Vertical { min, .. } => match min {
                NewlineMin::Zero => {}
                NewlineMin::One => {
                    self.out.newline()?;
                    wcx.is_whitespace_mode_out = true;
                }
            },
        }
        Ok(())
    }

    fn flush_whitespace(
        &self,
        wcx: &mut WhitespaceContext,
        len: usize,
        newlines: usize,
        is_comments_after: bool,
    ) -> FormatResult {
        self.source.advance(len);
        let is_by_comments = wcx.is_comments_before || is_comments_after;
        enum Out {
            Nothing,
            Newline { double: bool },
            Space,
        }
        let out = match wcx.mode {
            WhitespaceMode::Vertical { kind, min } => match newlines {
                _ if !is_by_comments && matches!(min, NewlineMin::Zero) => Out::Nothing,
                0 => {
                    if is_by_comments {
                        Out::Space
                    } else {
                        Out::Nothing
                    }
                }
                1 => Out::Newline { double: false },
                2.. => {
                    let double = kind.allow_blank_line(wcx.is_comments_before, is_comments_after);
                    Out::Newline { double }
                }
            },
            WhitespaceMode::Horizontal {
                error_on_newline: _,
                space,
            } => {
                assert!(!wcx.is_comments_before || newlines == 0);
                if space || is_by_comments {
                    Out::Space
                } else {
                    Out::Nothing
                }
            }
        };
        match out {
            Out::Newline { double } => {
                // todo handle this condition upstream
                let should_omit_newlines = self.out.len() == 0;
                if !should_omit_newlines {
                    self.out.newline()?;
                    if double {
                        self.out.newline()?;
                    }
                    if is_comments_after {
                        self.out.indent()?;
                    }
                }
                wcx.is_whitespace_mode_out = true;
            }
            Out::Space => {
                self.out.token(" ")?;
                if matches!(wcx.mode, WhitespaceMode::Horizontal { .. }) {
                    wcx.is_whitespace_mode_out = true;
                }
            }
            Out::Nothing => {}
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
