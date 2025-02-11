use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;
use rustc_lexer::TokenKind;

/// What whitespace do we want to print, and how to respond to comments
#[derive(Clone, Copy)]
pub enum WhitespaceMode {
    Horizontal {
        /// True if we want to print a space, otherwise we're just allowing for comments
        space: bool,
    },
    Vertical(NewlineKind),
}

// todo force blank lines between items?
#[derive(Clone, Copy)]
pub enum NewlineKind {
    /// Newline between items where a blank line is allowed. (e.g. between statements)
    Between,
    /// Newline at the beginning of a braced section. A blank line is allowed only below comments.
    Above,
    /// Newline at the end of a braced section. A blank line is allowed only above comments.
    Below,
    /// Newline in a place where extra blank lines should be trimmed away.
    Within,
    /// Only add a newline if there are comments, otherwise similar to Within.
    IfComments,
}

impl NewlineKind {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            NewlineKind::Between => true,
            NewlineKind::Above => is_comments_before,
            NewlineKind::Below => is_comments_after,
            NewlineKind::Within | NewlineKind::IfComments => {
                is_comments_before && is_comments_after
            }
        }
    }
}

struct WhitespaceContext {
    is_comments_before: bool,
    /// In horizontal mode, true if any whitespace or comments are emitted.
    /// In vertical mode, true if any newlines emitted.
    is_whitespace_mode_out: bool,
    mode: WhitespaceMode,
    /// Some when any amount of whitespace is seen and not yet printed
    whitespace_buffer: Option</*newline count*/ usize>,
}

impl SourceFormatter {
    // todo optimize no-op case?
    pub fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult<bool> {
        let wcx = &mut WhitespaceContext {
            is_comments_before: false,
            is_whitespace_mode_out: false,
            mode,
            whitespace_buffer: None,
        };
        let mut tokens = rustc_lexer::tokenize(self.source.remaining());
        loop {
            let next_char = self.source.remaining().chars().next();
            if !next_char.is_some_and(|c| c == '/' || rustc_lexer::is_whitespace(c)) {
                // save the tokenizer some work
                break;
            }
            let Some(token) = tokens.next() else { break };
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_whitespace(wcx, true)?;
                    self.constraints()
                        .max_width
                        .with_replaced(None, || self.copy(token.len as usize))?;
                    wcx.is_comments_before = true;
                    if matches!(wcx.mode, WhitespaceMode::Horizontal { .. }) {
                        wcx.is_whitespace_mode_out = true;
                    }
                }
                TokenKind::Whitespace => {
                    let token_str = &self.source.remaining()[..token.len as usize];
                    let newlines = token_str
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count();
                    wcx.whitespace_buffer = Some(newlines);
                    self.source.advance(token.len as usize);
                }
                _ => break,
            }
        }
        self.flush_whitespace(wcx, false)?;
        self.ensure_required_whitespace(wcx)?;

        Ok(wcx.is_whitespace_mode_out)
    }

    fn ensure_required_whitespace(&self, wcx: &mut WhitespaceContext) -> FormatResult {
        if wcx.is_whitespace_mode_out {
            return Ok(());
        }
        match wcx.mode {
            WhitespaceMode::Horizontal { space } => {
                if space {
                    self.out.token(" ")?;
                    wcx.is_whitespace_mode_out = true;
                }
            }
            WhitespaceMode::Vertical(kind) => {
                if !matches!(kind, NewlineKind::IfComments) {
                    self.out.newline()?;
                    wcx.is_whitespace_mode_out = true;
                }
            }
        }
        Ok(())
    }

    fn flush_whitespace(
        &self,
        wcx: &mut WhitespaceContext,
        is_comments_after: bool,
    ) -> FormatResult {
        let Some(newlines) = wcx.whitespace_buffer.take() else {
            return Ok(());
        };
        let is_by_comments = wcx.is_comments_before || is_comments_after;
        enum Out {
            Nothing,
            Newline { double: bool },
            Space,
        }
        let out = match (newlines, wcx.mode) {
            (_, WhitespaceMode::Vertical(NewlineKind::IfComments)) if !is_by_comments => {
                Out::Nothing
            }
            (2.., WhitespaceMode::Vertical(kind)) => {
                let double = kind.allow_blank_line(wcx.is_comments_before, is_comments_after);
                Out::Newline { double }
            }
            (1, WhitespaceMode::Vertical(_)) => Out::Newline { double: false },
            (1.., _) if is_by_comments => Out::Newline { double: false },
            _ if is_by_comments => Out::Space,
            (_, WhitespaceMode::Horizontal { space: true }) => Out::Space,
            _ => Out::Nothing,
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
