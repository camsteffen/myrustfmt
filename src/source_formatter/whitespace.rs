use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use rustc_lexer::TokenKind;

/// Answers the question: What whitespace to we expect to print, ignoring comments?
#[derive(Clone, Copy)]
pub enum WhitespaceMode {
    Newline(NewlineKind),
    Space,
    Void,
}

#[derive(Clone, Copy)]
pub enum NewlineKind {
    /// Newline between items where a blank line is allowed. (e.g. between statements)
    Between,
    /// Newline at the beginning of a braced section. A blank line is allowed only below comments.
    Leading,
    /// Newline at the end of a braced section. A blank line is allowed only above comments.
    Trailing,
    /// Newline in a less typical place where extra blank lines should be trimmed away.
    Split,
}

impl NewlineKind {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            NewlineKind::Between => true,
            NewlineKind::Leading => is_comments_before,
            NewlineKind::Trailing => is_comments_after,
            NewlineKind::Split => is_comments_before && is_comments_after,
        }
    }
}

pub fn handle_whitespace(mode: WhitespaceMode, sf: &SourceFormatter) -> FormatResult {
    WhitespaceContext {
        sf,
        is_comments_before: false,
        whitespace_buffer: None,
        mode,
        is_required_whitespace_out: false,
        is_after_line_comment_out: false,
    }
    .doit()
}

struct WhitespaceContext<'a> {
    sf: &'a SourceFormatter,
    mode: WhitespaceMode,
    /// Some if any amount of whitespace is seen and not yet printed
    whitespace_buffer: Option</*newline count*/ usize>,
    is_after_line_comment_out: bool,
    is_required_whitespace_out: bool,
    is_comments_before: bool,
}

impl WhitespaceContext<'_> {
    fn doit(&mut self) -> FormatResult {
        for token in rustc_lexer::tokenize(self.sf.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_whitespace(true)?;
                    self.sf
                        .constraints()
                        .with_no_max_width(|| self.sf.copy(token.len as usize))?;
                    self.is_comments_before = true;
                    self.is_after_line_comment_out =
                        matches!(token.kind, TokenKind::LineComment { .. });
                    if matches!(self.mode, WhitespaceMode::Space) {
                        // comments count for a space in this universe
                        self.is_required_whitespace_out = true;
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

        if !self.is_required_whitespace_out {
            match self.mode {
                WhitespaceMode::Newline(_) => self.sf.out.newline()?,
                WhitespaceMode::Space => self.sf.out.token(" ")?,
                WhitespaceMode::Void => {}
            }
        }
        self.sf.next_is_whitespace_or_comments.set(false);
        Ok(())
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
            (2.., WhitespaceMode::Newline(kind))
                if kind.allow_blank_line(self.is_comments_before, is_comments_after) =>
            {
                Out::Newline { double: true }
            }
            (1.., WhitespaceMode::Newline(_)) => Out::Newline { double: false },
            (1.., _) if is_by_comments => Out::Newline { double: false },
            _ if is_by_comments => Out::Space,
            (_, WhitespaceMode::Space) => Out::Space,
            _ => return Ok(()),
        };
        match out {
            Out::Newline { double } => {
                // todo handle this upstream
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
                self.is_required_whitespace_out = true;
            }
            Out::Space => {
                self.sf.out.token(" ")?;
                if matches!(self.mode, WhitespaceMode::Space) {
                    self.is_required_whitespace_out = true;
                }
            }
        }
        Ok(())
    }
}
