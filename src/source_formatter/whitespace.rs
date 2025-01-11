use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use rustc_lexer::TokenKind;

pub enum WhitespaceMode {
    Newline,
    Space,
    Token,
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
    fn flush_skipped_whitespace(&mut self, should_indent: bool) -> FormatResult {
        let Some(newlines) = self.skipped_whitespace.take() else {
            return Ok(());
        };
        if self.is_after_line_comment_out || matches!(self.mode, WhitespaceMode::Newline) {
            // todo detect start of blocks too
            if self.sf.out.len() > 0 {
                self.sf.out.newline()?;
                if matches!(self.mode, WhitespaceMode::Newline) && newlines > 1 {
                    self.sf.out.newline()?;
                }
                if should_indent {
                    self.sf.out.indent()?;
                }
            }
            self.is_required_whitespace_out = true;
        } else {
            self.sf.out.token(" ")?;
            if matches!(self.mode, WhitespaceMode::Space) {
                self.is_required_whitespace_out = true;
            }
        }
        Ok(())
    }

    fn doit(&mut self) -> FormatResult {
        for token in rustc_lexer::tokenize(self.sf.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.flush_skipped_whitespace(true)?;
                    self.sf
                        .constraints()
                        .with_no_max_width(|| self.sf.copy(token.len as usize))?;
                    self.twas_comments = true;
                    self.is_after_line_comment_out =
                        matches!(token.kind, TokenKind::LineComment { .. });
                    if matches!(self.mode, WhitespaceMode::Space) {
                        // comments can count for a space
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
        if matches!(self.mode, WhitespaceMode::Token) && !self.twas_comments {
            // ignore skipped (extra) whitespace
        } else {
            let should_indent = !matches!(self.mode, WhitespaceMode::Newline);
            self.flush_skipped_whitespace(should_indent)?
        }
        if !self.is_required_whitespace_out {
            match self.mode {
                WhitespaceMode::Newline => {
                    self.sf.out.newline()?;
                }
                WhitespaceMode::Space => self.sf.out.token(" ")?,
                WhitespaceMode::Token => {}
            }
        }
        self.sf.next_is_whitespace_or_comments.set(false);
        Ok(())
    }
}
