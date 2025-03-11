use std::cell::Cell;
use std::panic::Location;
use crate::constraint_writer::{ConstraintRecoveryMode, ConstraintWriter, ConstraintWriterCheckpoint, ConstraintWriterLookahead};
use crate::constraints::{OwnedConstraints};
use crate::error::FormatResult;
use crate::error_emitter::ErrorEmitter;
use self::source_reader::SourceReader;
use crate::util::chars::is_closer_char;
use rustc_span::{BytePos, Pos, Span};
use std::rc::Rc;

mod whitespace;
mod source_reader;

pub struct SourceFormatterCheckpoint {
    source_pos: BytePos,
    writer_checkpoint: ConstraintWriterCheckpoint,
}

pub struct SourceFormatterLookahead {
    source_pos: BytePos,
    writer_lookahead: ConstraintWriterLookahead,
}

pub struct SourceFormatter {
    source: SourceReader,
    out: ConstraintWriter,
    /// The number of spaces for the current level of indentation
    pub indent: Cell<u32>,
}

macro_rules! delegate_to_constraint_writer {
    ($($vis:vis fn $name:ident $(<$gen:tt>)?(&self $(, $arg:ident: $ty:ty)*) $(-> $ret_ty:ty)? ;)*) => {
        impl SourceFormatter {
            $($vis fn $name $(<$gen>)? (&self $(, $arg: $ty)*) $(-> $ret_ty)? {
                self.out.$name($($arg),*)
            })*
        }
    }
}

delegate_to_constraint_writer! {
    pub fn constraint_recovery_mode_max(&self, mode: ConstraintRecoveryMode) -> Option<impl Drop>;
    pub fn constraints(&self) -> &OwnedConstraints;
    pub fn current_max_width(&self) -> Option<u32>;
    pub fn enforce_max_width(&self) -> Option<impl Drop>;
    pub fn has_any_constraint_recovery(&self) -> bool;
    pub fn with_enforce_max_width<T>(&self, scope: impl FnOnce() -> T) -> T;
    // todo make sure any math using two values of this are guaranteed to be on the same line
    pub fn last_line_len(&self) -> u32;
    pub fn line(&self) -> u32;
    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T;
    pub fn with_taken_buffer(&self, f: impl FnOnce(&mut String));

}

impl SourceFormatter {
    pub fn new(
        source: Rc<String>,
        constraints: OwnedConstraints,
        error_emitter: Rc<ErrorEmitter>,
    ) -> SourceFormatter {
        let capacity = source.len() * 2;
        SourceFormatter {
            source: SourceReader::new(source),
            out: ConstraintWriter::new(constraints, error_emitter, capacity),
            indent: Cell::new(0),
        }
    }

    #[cfg(test)]
    pub fn new_defaults(source: impl Into<String>) -> SourceFormatter {
        Self::new(
            Rc::new(source.into()),
            OwnedConstraints::default(),
            Rc::new(ErrorEmitter::new(None)),
        )
    }

    pub fn finish(self) -> String {
        self.source.finish();
        self.out.finish()
    }

    pub fn checkpoint(&self) -> SourceFormatterCheckpoint {
        SourceFormatterCheckpoint {
            source_pos: self.source.pos.get(),
            writer_checkpoint: self.out.checkpoint(),
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &SourceFormatterCheckpoint) {
        let SourceFormatterCheckpoint {
            source_pos,
            ref writer_checkpoint,
        } = *checkpoint;
        self.out.restore_checkpoint(writer_checkpoint);
        self.source.pos.set(source_pos);
    }

    pub fn capture_lookahead(&self, from: &SourceFormatterCheckpoint) -> SourceFormatterLookahead {
        let writer_lookahead = self.out.capture_lookahead(&from.writer_checkpoint);
        let source_pos = self.source.pos.get();
        self.source.pos.set(from.source_pos);
        SourceFormatterLookahead {
            source_pos,
            writer_lookahead,
        }
    }

    pub fn restore_lookahead(&self, lookahead: &SourceFormatterLookahead) {
        self.out.restore_lookahead(&lookahead.writer_lookahead);
        self.source.pos.set(lookahead.source_pos);
    }

    pub fn line_col(&self) -> (u32, u32) {
        (self.out.line(), self.out.last_line_len())
    }

    pub fn pos(&self) -> usize {
        self.source.pos.get().to_usize()
    }

    pub fn source(&self) -> &str {
        &self.source.source
    }

    pub fn skip_token(&self, token: &str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source.eat(token)?;
        Ok(())
    }

    pub fn skip_token_if_present(&self, token: &str) -> FormatResult<bool> {
        // todo is this checkpoint avoidable?
        let checkpoint = self.checkpoint();
        let ws_result = self.horizontal_whitespace();
        if self.source.remaining().starts_with(token) {
            ws_result?;
            self.source.advance(token.len().try_into().unwrap());
            Ok(true)
        } else {
            self.restore_checkpoint(&checkpoint);
            Ok(false)
        }
    }

    pub fn copy_next_token(&self) -> FormatResult {
        self.horizontal_whitespace()?;
        let token = self.source.eat_next_token();
        self.out.token(token)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn eof(&self) -> FormatResult {
        self.source.expect_pos(BytePos::from_usize(self.source.source.len()))?;
        Ok(())
    }

    /// Write a token, asserting it is next in source.
    ///
    /// N.B. a token should not contain whitespace
    /// N.B. a token is indivisible (e.g. "::<" is two tokens since you can write it as "::  <")
    pub fn token(&self, token: &str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source.eat(token)?;
        self.out.token(token)?;
        Ok(())
    }

    /// Inserts a token without consuming it from source
    pub fn token_insert(&self, token: &str) -> FormatResult {
        self.out.token(token)?;
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

    /// Write a token that might be missing from source
    pub fn token_maybe_missing(&self, token: &str) -> FormatResult {
        self.skip_token_if_present(token)?;
        self.token_insert(token)?;
        Ok(())
    }

    /// Copy a token from source
    pub fn token_from_source(&self, span: Span) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source.expect_pos(span.lo())?;
        let token = self.source.get_span(span);
        self.token_unchecked(token)?;
        Ok(())
    }

    pub fn indent(&self) {
        self.out.spaces(self.indent.get());
    }

    fn copy(&self, len: u32) -> FormatResult {
        let segment = &self.source.remaining()[..len.try_into().unwrap()];
        self.out.write_possibly_multiline(segment)?;
        self.source.advance(len);
        Ok(())
    }

    pub fn copy_span(&self, span: Span) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source.expect_pos(span.lo())?;
        self.copy(span.hi().to_u32() - span.lo().to_u32())?;
        Ok(())
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&self, token: &str) -> FormatResult {
        self.out.token(token)?;
        self.source.advance(token.len().try_into().unwrap());
        Ok(())
    }

    pub fn last_line_is_closers(&self) -> bool {
        self.with_last_line(|line| {
            let after_indent = &line[self.indent.get().try_into().unwrap()..];
            after_indent.chars().all(is_closer_char)
        })
    }

    #[track_caller]
    pub fn debug_buffer(&self) {
        let location = Location::caller();
        self.with_taken_buffer(|b| {
            eprintln!("[{location}] buffer:\n{b}\n\n");
        });
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
        sf.newline_within_indent().unwrap();
        sf.token("aa").unwrap();
        sf.eof().unwrap();
        assert_eq!(sf.finish(), "aa\naa");
    }

    #[test]
    fn no_indent_for_blank_line() {
        let sf = SourceFormatter::new_defaults("aa\n    \naa");
        sf.indent.set(4);
        sf.token("aa").unwrap();
        sf.newline_within_indent().unwrap();
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
