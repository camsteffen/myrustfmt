use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use rustc_lexer::TokenKind;

#[derive(Clone, Copy)]
pub enum NewlineKind {
    /// Newline between items where a blank line is allowed.
    Between,
    /// Newline at the beginning of a braced section. A blank line is allowed only below comments.
    Leading,
    /// Newline at the end of a braced section. A blank line is allowed only above comments.
    Trailing,
    /// Newline in a less typical place where extra blank lines should be trimmed away.
    Split,
}

impl NewlineKind {
    pub fn allow_blank_line(self, is_before_comments: bool, is_after_comments: bool) -> bool {
        match self {
            NewlineKind::Between => true,
            NewlineKind::Leading => is_after_comments,
            NewlineKind::Trailing => is_before_comments,
            NewlineKind::Split => is_after_comments && is_before_comments,
        }
    }
}

#[derive(Clone, Copy)]
pub enum WhitespaceMode {
    Newline(NewlineKind),
    Space,
    Void,
}

pub fn handle_whitespace(mode: WhitespaceMode, sf: &SourceFormatter) -> FormatResult {
    WhitespaceContext {
        sf,
        is_after_comments: false,
        skipped_whitespace: None,
        mode,
        is_required_whitespace_out: false,
        is_after_line_comment_out: false,
    }
    .doit()
}

struct WhitespaceContext<'a> {
    sf: &'a SourceFormatter,
    mode: WhitespaceMode,
    skipped_whitespace: Option</*newline count*/ usize>,
    is_after_line_comment_out: bool,
    is_required_whitespace_out: bool,
    is_after_comments: bool,
}

impl WhitespaceContext<'_> {
    fn doit(&mut self) -> FormatResult {
        for token in rustc_lexer::tokenize(self.sf.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_skipped_whitespace(true)?;
                    self.sf
                        .constraints()
                        .with_no_max_width(|| self.sf.copy(token.len as usize))?;
                    self.is_after_comments = true;
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
                    self.skipped_whitespace = Some(newlines);
                    self.sf.source.advance(token.len as usize);
                }
                _ => break,
            }
        }

        self.flush_skipped_whitespace(false)?;

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

    fn flush_skipped_whitespace(&mut self, is_before_comments: bool) -> FormatResult {
        let Some(newlines) = self.skipped_whitespace.take() else {
            return Ok(());
        };
        enum ToFlush {
            Newline { double: bool },
            Space,
        }
        let is_by_comments = is_before_comments || self.is_after_comments;
        let to_flush = match (newlines, self.mode) {
            (2.., WhitespaceMode::Newline(kind))
                if kind.allow_blank_line(is_before_comments, self.is_after_comments) =>
            {
                ToFlush::Newline { double: true }
            }
            (1.., WhitespaceMode::Newline(_)) => ToFlush::Newline { double: false },
            (1.., _) if is_by_comments => ToFlush::Newline { double: false },
            _ if is_by_comments => ToFlush::Space,
            (_, WhitespaceMode::Space) => ToFlush::Space,
            _ => return Ok(())
        };
        // let to_flush = match self.mode {
        //     WhitespaceMode::Newline(kind) => {
        //         if newlines > 0 {
        //             let double = newlines > 1
        //                 && kind.allow_blank_line(is_before_comments, self.is_after_comments);
        //             ToFlush::Newline { double }
        //         } else if is_before_comments {
        //             ToFlush::Space
        //         } else {
        //             ToFlush::None
        //         }
        //     }
        //     WhitespaceMode::Space | WhitespaceMode::Void => {
        //         if (is_before_comments || self.is_after_comments) && newlines > 0 {
        //             ToFlush::Newline { double: false }
        //         } else if matches!(self.mode, WhitespaceMode::Void) {
        //             ToFlush::None
        //         } else {
        //             ToFlush::Space
        //         }
        //     }
        // };
        match to_flush {
            ToFlush::Newline { double } => {
                // todo handle this upstream
                let should_omit_newlines = self.sf.out.len() == 0;
                if !should_omit_newlines {
                    self.sf.out.newline()?;
                    if double {
                        self.sf.out.newline()?;
                    }
                    if is_before_comments {
                        self.sf.out.indent()?;
                    }
                }
                self.is_required_whitespace_out = true;
            }
            ToFlush::Space => {
                self.sf.out.token(" ")?;
                if matches!(self.mode, WhitespaceMode::Space) {
                    self.is_required_whitespace_out = true;
                }
            }
        }
        Ok(())
    }
}
