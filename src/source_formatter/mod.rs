use crate::constraint_writer::{
    ConstraintError, ConstraintWriter, NewlineNotAllowedError, TooWideError, WriterSnapshot,
};
use crate::constraints::Constraints;
use rustc_data_structures::sync::Lrc;
use rustc_errors::emitter::{HumanEmitter, stderr_destination};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_lexer::TokenKind;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use rustc_span::{
    BytePos, FileName, Pos, Span,
    source_map::{FilePathMapping, SourceMap},
};
use tracing::info;

pub struct SourceFormatterSnapshot {
    writer_snapshot: WriterSnapshot,
    pos: BytePos,
}

pub type FormatResult = Result<(), FormatError>;

#[derive(Clone, Copy, Debug)]
pub struct FormatError {
    kind: ConstraintError,
    pos: BytePos,
}

pub struct SourceFormatter<'a> {
    out: ConstraintWriter,
    source: &'a str,
    pos: BytePos,
    next_is_whitespace_or_comments: bool,
}

impl<'a> SourceFormatter<'a> {
    pub fn new(source: &'a str, constraints: Constraints) -> SourceFormatter<'a> {
        SourceFormatter {
            out: ConstraintWriter::new(constraints),
            source,
            pos: BytePos(0),
            next_is_whitespace_or_comments: true,
        }
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn constraints(&mut self) -> &mut Constraints {
        self.out.constraints()
    }

    pub fn snapshot(&self) -> SourceFormatterSnapshot {
        SourceFormatterSnapshot {
            writer_snapshot: self.out.snapshot(),
            pos: self.pos,
        }
    }

    pub fn restore(&mut self, snapshot: &SourceFormatterSnapshot) {
        self.pos = snapshot.pos;
        self.out.restore(&snapshot.writer_snapshot);
    }

    pub fn last_line_width(&self) -> usize {
        self.out.last_line_width()
    }

    /** Writes a newline character and indent characters according to the current indent level */
    pub fn newline_indent(&mut self) -> FormatResult {
        self.handle_whitespace_and_comments();
        self.out
            .newline()
            .map_err(|e| self.lift_constraint_err(e))?;
        self.out.indent().map_err(|e| self.lift_constraint_err(e))?;
        Ok(())
    }

    /** Writes a space and accounts for spaces and comments in source */
    pub fn space(&mut self) -> FormatResult {
        if !self.handle_whitespace_and_comments() {
            self.out
                .token(" ")
                .map_err(|e| self.lift_constraint_err(e))?;
        }
        Ok(())
    }

    /** Convenience for calling `token_at` followed by `space` */
    pub fn token_at_space(&mut self, token: &'static str, pos: BytePos) -> FormatResult {
        self.token_at(token, pos)?;
        self.space()?;
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given position */
    pub fn token_at(&mut self, token: &str, pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        assert_eq!(pos, self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given ending position */
    pub fn token_end_at(&mut self, token: &str, end_pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    /**
     * Write a token, asserting it is next in source.
     *
     * Note: This compares the token string to source and is thus somewhat less
     * performant than token_at. But this is useful when you don't have a Span.
     */
    pub fn token_expect(&mut self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        if !self.source[self.pos.to_usize()..].starts_with(token) {
            panic!(
                "expected token \"{}\", found \"{}\"",
                token,
                &self.source
                    [self.pos.to_usize()..(self.pos.to_usize() + 10).min(self.source.len())]
            );
        }
        self.token_unchecked(token)?;
        Ok(())
    }

    /** Write a token that may be next in source, or otherwise is missing */
    pub fn token_maybe_missing(&mut self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        if self.source[self.pos.to_usize()..].starts_with(token) {
            self.token_unchecked(token)
        } else {
            self.token_missing(token)
        }
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&mut self, token: &str) -> FormatResult {
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        self.next_is_whitespace_or_comments = true;
        Ok(())
    }

    /** Write a token assuming it is missing from source */
    fn token_missing(&mut self, token: &str) -> FormatResult {
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        Ok(())
    }

    pub fn token_from_source(&mut self, span: Span) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        assert_eq!(span.lo(), self.pos);
        let token = self.expect_span(span);
        self.token_unchecked(token)?;
        Ok(())
    }

    pub fn expect_span(&self, span: Span) -> &'a str {
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }

    pub fn lift_constraint_err(&self, out_err: impl Into<ConstraintError>) -> FormatError {
        FormatError {
            kind: out_err.into(),
            pos: self.pos,
        }
    }

    fn handle_whitespace_and_comments(&mut self) -> bool {
        let mut len = 0;
        let mut len_without_trailing_whitespace = 0;
        for token in rustc_lexer::tokenize(&self.source[self.pos.to_usize()..]) {
            match token.kind {
                |TokenKind::LineComment { .. }| TokenKind::BlockComment { .. } => {
                    len += token.len as usize;
                    len_without_trailing_whitespace = len;
                }
                | TokenKind::Whitespace => {
                    len += token.len as usize;
                }
                _ => break,
            }
        }
        if len_without_trailing_whitespace > 0 {
            self.copy(len_without_trailing_whitespace);
        }
        self.pos = self.pos + BytePos::from_usize(len - len_without_trailing_whitespace);
        self.next_is_whitespace_or_comments = false;
        len_without_trailing_whitespace > 0
    }

    fn handle_whitespace_and_comments_if_needed(&mut self) {
        if self.next_is_whitespace_or_comments {
            self.handle_whitespace_and_comments();
        }
    }
    
    fn copy(&mut self, len: usize) {
        let segment = &self.source
            [self.pos.to_usize()..self.pos.to_usize() + len];
        self.out.write_unchecked(segment);
        self.pos = self.pos + BytePos::from_usize(len);
    }

    pub fn debug_pos(&self) {
        info!(
            "{:?}",
            self.source[self.pos.to_usize()..].chars().next().unwrap()
        );
    }
}
