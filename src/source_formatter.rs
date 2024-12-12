use crate::constraint_writer::{ConstraintError, ConstraintWriter, WriterSnapshot};
use crate::constraints::Constraints;
use crate::source_reader::SourceReader;

use rustc_lexer::TokenKind;
use rustc_span::{BytePos, Pos, Span};

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
    source: SourceReader<'a>,
    out: ConstraintWriter,
    next_is_whitespace_or_comments: bool,
}

impl<'a> SourceFormatter<'a> {
    pub fn new(source: &'a str, constraints: Constraints) -> SourceFormatter<'a> {
        SourceFormatter {
            source: SourceReader::new(source),
            out: ConstraintWriter::new(constraints),
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
            pos: self.source.pos,
        }
    }

    pub fn restore(&mut self, snapshot: &SourceFormatterSnapshot) {
        self.source.pos = snapshot.pos;
        self.out.restore(&snapshot.writer_snapshot);
    }

    pub fn require_width(&mut self, len: usize) -> FormatResult {
        self.out.require_width(len).map_err(|e| self.lift_constraint_err(e))
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

    pub fn char_ending_at(&self, pos: BytePos) -> u8 {
        self.source.source.as_bytes()[pos.to_usize() - 1]
    }

    pub fn skip_token_if_present(&mut self, token: &str) {
        self.handle_whitespace_and_comments_if_needed();
        if self.source.remaining().starts_with(token) {
            self.source.advance(token.len());
            self.next_is_whitespace_or_comments = true;
        }
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

    /** Write a token, asserting it is next in source and has the given position */
    pub fn token_at(&mut self, token: &str, pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        self.source.expect_pos(pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given ending position */
    pub fn token_end_at(&mut self, token: &str, end_pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        self.token_unchecked(token)?;
        self.source.expect_pos(end_pos);
        Ok(())
    }

    /** Convenience for calling `token_at` followed by `space` */
    pub fn token_at_space(&mut self, token: &'static str, pos: BytePos) -> FormatResult {
        self.token_at(token, pos)?;
        self.space()?;
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
        if !self.source.remaining().starts_with(token) {
            panic!(
                "expected token {:?}, found {:?}",
                token,
                &self.source.remaining()[..10.min(self.source.remaining().len())]
            );
        }
        self.token_unchecked(token)?;
        Ok(())
    }

    /** Write a token that may be next in source, or otherwise is missing from source */
    pub fn token_maybe_missing(&mut self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        if self.source.remaining().starts_with(token) {
            self.token_unchecked(token)
        } else {
            self.token_missing(token)
        }
    }

    /** Copy a token from source */
    pub fn token_from_source(&mut self, span: Span) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed();
        self.source.expect_pos(span.lo());
        let token = self.source.get_span(span);
        self.token_unchecked(token)?;
        Ok(())
    }

    pub fn lift_constraint_err(&self, out_err: impl Into<ConstraintError>) -> FormatError {
        FormatError {
            kind: out_err.into(),
            pos: self.source.pos,
        }
    }

    fn handle_whitespace_and_comments(&mut self) -> bool {
        let mut len = 0;
        let mut len_without_trailing_whitespace = 0;
        for token in rustc_lexer::tokenize(self.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    len += token.len as usize;
                    len_without_trailing_whitespace = len;
                }
                TokenKind::Whitespace => {
                    len += token.len as usize;
                }
                _ => break,
            }
        }
        if len_without_trailing_whitespace > 0 {
            self.copy(len_without_trailing_whitespace);
        }
        self.source.advance(len - len_without_trailing_whitespace);
        self.next_is_whitespace_or_comments = false;
        len_without_trailing_whitespace > 0
    }

    fn handle_whitespace_and_comments_if_needed(&mut self) {
        if self.next_is_whitespace_or_comments {
            self.handle_whitespace_and_comments();
        }
    }

    fn copy(&mut self, len: usize) {
        let segment = &self.source.remaining()[..len];
        self.out.write_unchecked(segment);
        self.source.advance(len);
    }

    pub fn copy_span(&mut self, span: Span) {
        self.handle_whitespace_and_comments_if_needed();
        self.source.expect_pos(span.lo());
        self.copy(span.hi().to_usize() - span.lo().to_usize())
    }

    pub fn copy_to(&mut self, pos: BytePos) {
        self.copy(pos.to_usize() - self.source.pos.to_usize());
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&mut self, token: &str) -> FormatResult {
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        self.source.advance(token.len());
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
}
