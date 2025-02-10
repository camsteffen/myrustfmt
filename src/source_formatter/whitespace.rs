use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;
use rustc_lexer::TokenKind;

/// Answers the question: What whitespace to we expect to print, ignoring comments?
#[derive(Clone, Copy)]
pub enum WhitespaceMode {
    Horizontal { space: bool },
    Vertical(NewlineKind),
}

// todo force blank lines between items?
/// A bespoke way of characterizing line breaks that determines where blank lines are allowed
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
    /// Same as Within, but don't print a newline if there are no comments
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

pub fn handle_whitespace(mode: WhitespaceMode, sf: &SourceFormatter) -> FormatResult<bool> {
    WhitespaceContext {
        sf,
        is_comments_before: false,
        whitespace_buffer: None,
        mode,
        is_whitespace_mode_out: false,
    }
    .doit()
}

struct WhitespaceContext<'a> {
    sf: &'a SourceFormatter,
    mode: WhitespaceMode,
    /// Some when any amount of whitespace is seen and not yet printed
    whitespace_buffer: Option</*newline count*/ usize>,
    /// In horizontal mode, true if any whitespace or comments are emitted.
    /// In vertical mode, true if any newlines emitted.
    is_whitespace_mode_out: bool,
    is_comments_before: bool,
}

impl WhitespaceContext<'_> {
    fn doit(&mut self) -> FormatResult<bool> {
        for token in rustc_lexer::tokenize(self.sf.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_whitespace(true)?;
                    self.sf
                        .constraints()
                        .max_width
                        .with_replaced(None, || self.sf.copy(token.len as usize))?;
                    self.is_comments_before = true;
                    if matches!(self.mode, WhitespaceMode::Horizontal { .. }) {
                        self.is_whitespace_mode_out = true;
                    }
                }
                TokenKind::Whitespace => {
                    let token_str = &self.sf.source.remaining()[..token.len as usize];
                    let newlines = token_str.bytes().filter(|&b| b == b'\n').count();
                    self.whitespace_buffer = Some(newlines);
                    self.sf.source.advance(token.len as usize);
                }
                _ => break,
            }
        }

        self.flush_whitespace(false)?;

        let is_whitespace_required = match self.mode {
            WhitespaceMode::Horizontal { space } => space,
            WhitespaceMode::Vertical(NewlineKind::IfComments) => false,
            WhitespaceMode::Vertical(_) => true,
        };
        if is_whitespace_required && !self.is_whitespace_mode_out {
            match self.mode {
                WhitespaceMode::Vertical(_) => self.sf.out.newline()?,
                WhitespaceMode::Horizontal { .. } => self.sf.out.token(" ")?,
            }
        }
        self.sf.next_is_whitespace_or_comments.set(false);
        Ok(self.is_whitespace_mode_out)
    }

    fn flush_whitespace(&mut self, is_comments_after: bool) -> FormatResult {
        let Some(newlines) = self.whitespace_buffer.take() else {
            return Ok(());
        };
        let is_by_comments = self.is_comments_before || is_comments_after;
        enum Out {
            Newline { double: bool },
            Space,
        }
        let out = match (newlines, self.mode) {
            (_, WhitespaceMode::Vertical(NewlineKind::IfComments)) if !is_by_comments => {
                return Ok(());
            }
            (2.., WhitespaceMode::Vertical(kind)) => {
                let double = kind.allow_blank_line(self.is_comments_before, is_comments_after);
                Out::Newline { double }
            }
            (1.., WhitespaceMode::Vertical(_)) => Out::Newline { double: false },
            (1.., _) if is_by_comments => Out::Newline { double: false },
            _ if is_by_comments => Out::Space,
            (_, WhitespaceMode::Horizontal { space: true }) => Out::Space,
            _ => return Ok(()),
        };
        match out {
            Out::Newline { double } => {
                // todo handle this condition upstream
                let should_omit_newlines = self.sf.out.len() == 0;
                if !should_omit_newlines {
                    self.sf.out.newline()?;
                    if double {
                        self.sf.out.newline()?;
                    }
                    if is_comments_after {
                        self.sf.out.indent()?;
                    }
                }
                self.is_whitespace_mode_out = true;
            }
            Out::Space => {
                self.sf.out.token(" ")?;
                if matches!(self.mode, WhitespaceMode::Horizontal { .. }) {
                    self.is_whitespace_mode_out = true;
                }
            }
        }
        Ok(())
    }
}
