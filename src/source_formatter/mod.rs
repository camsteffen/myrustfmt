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
}

impl<'a> SourceFormatter<'a> {
    pub fn new(source: &'a str, constraints: Constraints) -> SourceFormatter<'a> {
        SourceFormatter {
            out: ConstraintWriter::new(constraints),
            source,
            pos: BytePos(0),
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
        self.no_space();
        self.out
            .newline()
            .map_err(|e| self.lift_constraint_err(e))?;
        self.out.indent().map_err(|e| self.lift_constraint_err(e))?;
        Ok(())
    }

    /** Writes a space and accounts for spaces and comments in source */
    pub fn space(&mut self) -> FormatResult {
        self.out
            .token(" ")
            .map_err(|e| self.lift_constraint_err(e))?;
        self.skip_whitespace_and_comments();
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
        assert_eq!(pos, self.pos);
        self.token_expect(token)?;
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given ending position */
    pub fn token_end_at(&mut self, token: &str, end_pos: BytePos) -> FormatResult {
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_expect(token)?;
        Ok(())
    }

    /**
     * Write a token, asserting it is next in source.
     *
     * Note: This compares the token string to source and is thus somewhat less
     * performant than token_at. But this is useful when you don't have a Span.
     */
    pub fn token_expect(&mut self, token: &str) -> FormatResult {
        if !self.source[self.pos.to_usize()..].starts_with(token) {
            panic!(
                "expected token \"{}\", found \"{}\"",
                token,
                &self.source
                    [self.pos.to_usize()..(self.pos.to_usize() + 10).min(self.source.len())]
            );
        }
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    /** Write a token that may be next in source, or otherwise is missing */
    pub fn token_maybe_missing(&mut self, token: &str) -> FormatResult {
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
        assert_eq!(span.lo(), self.pos);
        let token = self.expect_span(span);
        self.token_expect(token)?;
        Ok(())
    }

    pub fn expect_span(&self, span: Span) -> &'a str {
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }

    pub fn optional_space(&mut self, is_space: bool) -> FormatResult {
        if is_space {
            self.space()?;
        } else {
            self.no_space();
        }
        Ok(())
    }

    pub fn lift_constraint_err(&self, out_err: impl Into<ConstraintError>) -> FormatError {
        FormatError {
            kind: out_err.into(),
            pos: self.pos,
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        rustc_lexer::tokenize(&self.source[self.pos.to_usize()..])
            .take_while(|token| {
                matches!(
                    token.kind,
                    |TokenKind::LineComment { .. }| TokenKind::BlockComment { .. }
                        | TokenKind::Whitespace
                )
            })
            .for_each(|token| {
                info!("skipping whitespace: {}", token.len);
                self.pos = self.pos + BytePos::from_u32(token.len);
            });
    }

    pub fn no_space(&mut self) {
        self.skip_whitespace_and_comments();
    }

    pub fn debug_pos(&self) {
        info!(
            "{:?}",
            self.source[self.pos.to_usize()..].chars().next().unwrap()
        );
    }
}
