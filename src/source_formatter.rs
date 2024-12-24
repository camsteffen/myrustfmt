use crate::constraint_writer::{ConstraintWriter, ConstraintWriterSnapshot};
use crate::constraints::Constraints;
use crate::error::{FormatError, FormatResult, NewlineNotAllowedError, WidthLimitExceededError};
use crate::source_reader::SourceReader;
use rustc_lexer::TokenKind;
use rustc_span::{BytePos, Pos, Span};
use std::cell::Cell;
use tracing::info;

pub struct SourceFormatterSnapshot {
    writer_snapshot: ConstraintWriterSnapshot,
    pos: BytePos,
    next_is_whitespace_or_comments: bool,
}

pub struct SourceFormatter {
    source: SourceReader,
    out: ConstraintWriter,
    next_is_whitespace_or_comments: Cell<bool>,
}

impl SourceFormatter {
    pub fn new(source: String, constraints: Constraints) -> SourceFormatter {
        SourceFormatter {
            source: SourceReader::new(source),
            out: ConstraintWriter::new(constraints),
            next_is_whitespace_or_comments: Cell::new(true),
        }
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

    pub fn line(&self) -> usize {
        self.out.line()
    }

    pub fn pos(&self) -> usize {
        self.source.pos.get().to_usize()
    }

    /** Writes a newline character and indent characters according to the current indent level */
    pub fn newline_indent(&self) -> FormatResult {
        self.handle_whitespace_and_comments_for_newline()
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
            self.handle_whitespace_and_comments()?;
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
    pub fn eat_token(&self) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        let token = self.source.eat_next_token();
        self.token_out(token)?;
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    /** Writes a space and accounts for spaces and comments in source */
    pub fn space(&self) -> FormatResult {
        if !self.handle_whitespace_and_comments()? {
            self.out.token(" ")?;
        }
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given position */
    pub fn token_at(&self, token: &str, pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.expect_pos(pos)?;
        self.token_unchecked(token)?;
        Ok(())
    }

    /** Write a token, asserting it is next in source and has the given ending position */
    pub fn token_end_at(&self, token: &str, end_pos: BytePos) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.token_unchecked(token)?;
        self.source.expect_pos(end_pos)?;
        Ok(())
    }

    /** Convenience for calling `token_at` followed by `space` */
    pub fn token_at_space(&self, token: &'static str, pos: BytePos) -> FormatResult {
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
    pub fn token_expect(&self, token: &str) -> FormatResult {
        self.handle_whitespace_and_comments_if_needed()?;
        self.source.eat(token)?;
        self.next_is_whitespace_or_comments.set(true);
        self.token_out(token)?;
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

    pub fn require_width(&self, width: usize) -> Result<(), FormatError> {
        self.out
            .remaining_width()
            .and_then(|remaining| match remaining {
                Some(r) if r < width => Err(WidthLimitExceededError),
                _ => Ok(()),
            })?;
        Ok(())
    }
    
    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.out.with_last_line(f)
    }

    fn handle_whitespace_and_comments(&self) -> FormatResult<bool> {
        let mut whitespace_len = 0usize;
        let mut comments_happened = false;
        for token in rustc_lexer::tokenize(self.source.remaining()) {
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.constraints()
                        .with_no_width_limit(|| self.copy(whitespace_len + token.len as usize))?;
                    whitespace_len = 0;
                    comments_happened = true;
                }
                TokenKind::Whitespace => {
                    whitespace_len += token.len as usize;
                }
                _ => break,
            }
        }
        // skip trailing whitespace
        self.source.advance(whitespace_len);
        self.next_is_whitespace_or_comments.set(false);
        Ok(comments_happened)
    }

    fn handle_whitespace_and_comments_for_newline(&self) -> FormatResult {
        let mut newlines_happened = false;
        for token in rustc_lexer::tokenize(self.source.remaining()) {
            let token_str = &self.source.remaining()[..token.len as usize];
            match token.kind {
                TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => {
                    self.constraints()
                        .with_no_width_limit(|| self.copy(token.len as usize))?;
                }
                TokenKind::Whitespace => {
                    let newline_count = token_str.bytes().filter(|&b| b == b'\n').count();
                    self.newline()?;
                    if newline_count >= 2 {
                        self.newline()?;
                    }
                    self.indent()?;
                    self.source.advance(token.len as usize);
                    newlines_happened = true;
                }
                _ => break,
            }
        }
        if !newlines_happened {
            self.newline()?;
            self.indent()?;
        }
        Ok(())
    }

    fn handle_whitespace_and_comments_if_needed(&self) -> FormatResult {
        if self.next_is_whitespace_or_comments.get() {
            self.handle_whitespace_and_comments()?;
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

    /** Write a token */
    fn token_out(&self, token: &str) -> FormatResult {
        self.out.token(&token)?;
        Ok(())
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&self, token: &str) -> FormatResult {
        self.token_out(token)?;
        self.source.advance(token.len());
        self.next_is_whitespace_or_comments.set(true);
        Ok(())
    }

    /** Write a token assuming it is missing from source */
    pub fn token_missing(&self, token: &str) -> FormatResult {
        self.token_out(token)
    }
}
