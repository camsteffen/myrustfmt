use crate::constraint_writer::{ConstraintWriter, ConstraintWriterSnapshot};
use crate::constraints::Constraints;
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::source_formatter::whitespace::{WhitespaceMode, handle_whitespace};
use crate::source_reader::SourceReader;
use rustc_span::{BytePos, Pos, Span};
use std::cell::Cell;

mod whitespace;

pub struct SourceFormatterSnapshot {
    writer_snapshot: ConstraintWriterSnapshot,
    pos: BytePos,
    next_is_whitespace_or_comments: bool,
}

pub struct SourceFormatter {
    source: SourceReader,
    out: ConstraintWriter,
    // todo move to SourceReader
    next_is_whitespace_or_comments: Cell<bool>,
}

impl SourceFormatter {
    pub fn new(
        source: impl Into<String>,
        constraints: Constraints,
    ) -> SourceFormatter {
        SourceFormatter {
            source: SourceReader::new(source.into()),
            out: ConstraintWriter::new(constraints),
            next_is_whitespace_or_comments: Cell::new(true),
        }
    }

    pub fn new_defaults(source: impl Into<String>) -> SourceFormatter {
        Self::new(source, Constraints::default())
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn snapshot(&self) -> SourceFormatterSnapshot {
        let Self {
            out,
            source,
            next_is_whitespace_or_comments,
        } = self;
        SourceFormatterSnapshot {
            writer_snapshot: out.snapshot(),
            pos: source.pos.get(),
            next_is_whitespace_or_comments: next_is_whitespace_or_comments.get(),
        }
    }

    pub fn restore(&self, snapshot: &SourceFormatterSnapshot) {
        let SourceFormatterSnapshot {
            ref writer_snapshot,
            pos,
            next_is_whitespace_or_comments,
        } = *snapshot;
        self.source.pos.set(pos);
        self.out.restore(writer_snapshot);
        self.next_is_whitespace_or_comments
            .set(next_is_whitespace_or_comments);
    }

    pub fn last_line_len(&self) -> usize {
        self.out.last_line_len()
    }

    pub fn len(&self) -> usize {
        self.out.len()
    }

    pub fn line(&self) -> u32 {
        self.out.line()
    }

    pub fn pos(&self) -> usize {
        self.source.pos.get().to_usize()
    }

    /** Writes a newline character and indent characters according to the current indent level */
    pub fn newline_indent(&self) -> FormatResult {
        self.handle_whitespace_and_comments(WhitespaceMode::Newline)
    }

    pub fn char_ending_at(&self, pos: BytePos) -> u8 {
        self.source.source.as_bytes()[pos.to_usize() - 1]
    }

    pub fn skip_token(&self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.eat(token)?;
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    pub fn skip_token_if_present(&self, token: &str) -> FormatResult {
        let snapshot;
        if self.next_is_whitespace_or_comments.get() {
            snapshot = Some(self.snapshot());
            self.handle_whitespace_and_comments(WhitespaceMode::Token)?;
        } else {
            snapshot = None;
        }
        // self.handle_whitespace_and_comments_if_needed()?;
        if self.source.remaining().starts_with(token) {
            self.source.advance(token.len());
            self.next_is_whitespace_or_comments.set(true);
        } else if let Some(snapshot) = snapshot {
            self.restore(&snapshot);
        }
        Ok(())
    }

    /** Writes a space and accounts for spaces and comments in source */
    pub fn copy_next_token(&self) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        let token = self.source.eat_next_token();
        self.out.token(&token)?;
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    pub fn eof(&self) -> FormatResult {
        self.source
            .expect_pos(BytePos::from_usize(self.source.source.len()))?;
        Ok(())
    }

    /**
     * Write a token, asserting it is next in source.
     *
     * N.B. a token should not contain whitespace
     * N.B. a token is indivisible (e.g. "::<" is two tokens since you can write "::  <")
     */
    pub fn token(&self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.eat(token)?;
        self.next_is_whitespace_or_comments.set(true);
        self.out.token(&token)?;
        Ok(())
    }

    /** Writes a space and accounts for spaces and comments in source */
    // todo do newlines and comments sneak in when it should be single line?
    pub fn space(&self) -> FormatResult {
        if self.next_is_whitespace_or_comments.get() {
            self.handle_whitespace_and_comments(WhitespaceMode::Space)?;
        } else {
            self.out.token(" ")?;
        }
        Ok(())
    }

    pub fn token_space(&self, token: &str) -> FormatResult {
        self.token(token)?;
        self.space()?;
        Ok(())
    }

    pub fn space_token_space(&self, token: &str) -> FormatResult {
        self.space()?;
        self.token(token)?;
        self.space()?;
        Ok(())
    }

    pub fn space_token(&self, token: &str) -> FormatResult {
        self.space()?;
        self.token(token)?;
        Ok(())
    }

    /** Write a token that may be next in source, or otherwise is missing from source */
    pub fn token_maybe_missing(&self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        if self.source.remaining().starts_with(token) {
            self.token_unchecked(token)
        } else {
            self.token_missing(token)
        }
    }

    /** Copy a token from source */
    pub fn token_from_source(&self, span: Span) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.expect_pos(span.lo())?;
        let token = self.source.get_span(span);
        self.token_unchecked(token)?;
        Ok(())
    }

    pub fn require_width(&self, width: u32) -> Result<(), WidthLimitExceededError> {
        if let Some(remaining) = self.out.remaining_width() {
            let remaining = remaining?;
            if remaining < width {
                return Err(WidthLimitExceededError);
            }
        }
        Ok(())
    }

    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.out.with_last_line(f)
    }

    fn handle_whitespace_and_comments(&self, mode: WhitespaceMode) -> FormatResult {
        handle_whitespace(mode, self)
    }

    fn handle_whitespace_and_comments_if_needed(&self) -> FormatResult {
        if self.next_is_whitespace_or_comments.get() {
            self.handle_whitespace_and_comments(WhitespaceMode::Token)?;
        }
        Ok(())
    }

    fn newline(&self) -> FormatResult {
        self.out.newline()?;
        Ok(())
    }

    fn indent(&self) -> FormatResult {
        self.out.indent()?;
        Ok(())
    }

    fn copy(&self, len: usize) -> FormatResult {
        let segment = &self.source.remaining()[..len];
        self.out.write_possibly_multiline(segment)?;
        self.source.advance(len);
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    pub fn copy_span(&self, span: Span) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.expect_pos(span.lo())?;
        self.copy(span.hi().to_usize() - span.lo().to_usize())?;
        Ok(())
    }

    pub fn copy_to(&self, pos: BytePos) -> FormatResult {
        self.copy(pos.to_usize() - self.source.pos.get().to_usize())
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&self, token: &str) -> FormatResult {
        self.out.token(&token)?;
        self.source.advance(token.len());
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    /** Write a token assuming it is missing from source */
    pub fn token_missing(&self, token: &str) -> FormatResult {
        self.out.token(&token)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_token_skips_initial_whitespace() {
        let sf = SourceFormatter::new_defaults(" \naa");
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa");
    }

    #[test]
    fn replace_space_with_newline() {
        let sf = SourceFormatter::new_defaults("aa aa");
        sf.token("aa").unwrap();
        sf.newline_indent().unwrap();
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa\naa");
    }

    #[test]
    fn no_indent_for_blank_line() {
        let sf = SourceFormatter::new_defaults("aa\n    \naa");
        sf.constraints().increment_indent();
        sf.token("aa").unwrap();
        sf.newline_indent().unwrap();
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa\n\n    aa");
    }

    #[test]
    fn space_without_comment() {
        let sf = SourceFormatter::new_defaults("aa bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa bb");
    }

    #[test]
    fn space_missing_from_source() {
        let sf = SourceFormatter::new_defaults("aa,bb");
        sf.token("aa").unwrap();
        sf.token(",").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa, bb");
    }

    #[test]
    fn space_with_comment_no_space() {
        let sf = SourceFormatter::new_defaults("aa/*comment*/bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa/*comment*/bb");
    }

    #[test]
    fn space_with_comment_leading_space() {
        let sf = SourceFormatter::new_defaults("aa /*comment*/bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa /*comment*/bb");
    }

    #[test]
    fn space_with_comment_trailing_space() {
        let sf = SourceFormatter::new_defaults("aa/*comment*/ bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa/*comment*/ bb");
    }

    #[test]
    fn space_with_comment_space_on_both_sides() {
        let sf = SourceFormatter::new_defaults("aa /*comment*/ bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa /*comment*/ bb");
    }

    #[test]
    fn space_with_comment_extra_spaces_trimmed() {
        let sf = SourceFormatter::new_defaults("aa   /*comment*/   bb");
        sf.token("aa").unwrap();
        sf.space().unwrap();
        sf.token("bb").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa /*comment*/ bb");
    }

    #[test]
    fn space_around_comments_preserved_even_with_no_space_out() {
        let sf = SourceFormatter::new_defaults("( /*comment*/ aa");
        sf.token("(").unwrap();
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "( /*comment*/ aa");
    }

    #[test]
    fn newlines_removed_between_tokens() {
        let sf = SourceFormatter::new_defaults("(\naa");
        sf.token("(").unwrap();
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "(aa");
    }
}
