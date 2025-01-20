use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use rustc_lexer::TokenKind;

pub enum WhitespaceMode {
    Newline,
    NewlineSplit,
    /// Used at the beginning of a block and enforces that a block may not begin with a blank line
    NewlineLeading,
    /// Opposite of NewlineLeading - enforces that a block may not end with a blank line
    NewlineTrailing,
    Space,
    Void,
}

pub fn handle_whitespace(mode: WhitespaceMode, sf: &SourceFormatter) -> FormatResult {
    WhitespaceContext {
        sf,
        twas_comments: false,
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
    twas_comments: bool,
}

impl WhitespaceContext<'_> {
    fn flush_skipped_whitespace(&mut self, is_end: bool, should_indent: bool) -> FormatResult {
        let Some(newlines) = self.skipped_whitespace.take() else {
            return Ok(());
        };
        let is_newlines = if self.is_after_line_comment_out {
            true
        } else {
            match self.mode {
                WhitespaceMode::Space | WhitespaceMode::Void => false,
                WhitespaceMode::Newline
                | WhitespaceMode::NewlineLeading
                | WhitespaceMode::NewlineTrailing
                | WhitespaceMode::NewlineSplit => true,
            }
        };
        if is_newlines {
            // todo handle this upstream
            let should_omit_newlines = self.sf.out.len() == 0;
            if !should_omit_newlines {
                self.sf.out.newline()?;
                if newlines > 1 {
                    let allow_blank_line = match self.mode {
                        WhitespaceMode::Newline => true,
                        WhitespaceMode::NewlineLeading => self.twas_comments,
                        WhitespaceMode::NewlineTrailing => !is_end,
                        WhitespaceMode::NewlineSplit => self.twas_comments && !is_end,
                        WhitespaceMode::Space | WhitespaceMode::Void => false,
                    };
                    if allow_blank_line {
                        self.sf.out.newline()?;
                    }
                }
                if should_indent {
                    self.sf.out.indent()?;
                }
            }
            self.is_required_whitespace_out = true;
        } else {
            if !matches!(self.mode, WhitespaceMode::Void) {
                self.sf.out.token(" ")?;
                if matches!(self.mode, WhitespaceMode::Space) {
                    self.is_required_whitespace_out = true;
                }
            }
        }
        Ok(())
    }

    fn doit(&mut self) -> FormatResult {
        for token in rustc_lexer::tokenize(self.sf.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_skipped_whitespace(false, true)?;
                    self.sf
                        .constraints()
                        .with_no_max_width(|| self.sf.copy(token.len as usize))?;
                    self.twas_comments = true;
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

        // todo TDD
        let should_indent = match self.mode {
            WhitespaceMode::Space | WhitespaceMode::Void => true,
            WhitespaceMode::Newline
            | WhitespaceMode::NewlineLeading
            | WhitespaceMode::NewlineTrailing
            | WhitespaceMode::NewlineSplit => false,
        };
        self.flush_skipped_whitespace(true, should_indent)?;

        if !self.is_required_whitespace_out {
            match self.mode {
                WhitespaceMode::Newline
                | WhitespaceMode::NewlineLeading
                | WhitespaceMode::NewlineTrailing
                | WhitespaceMode::NewlineSplit => {
                    self.sf.out.newline()?;
                }
                WhitespaceMode::Space => self.sf.out.token(" ")?,
                WhitespaceMode::Void => {}
            }
        }
        self.sf.next_is_whitespace_or_comments.set(false);
        Ok(())
    }
}
