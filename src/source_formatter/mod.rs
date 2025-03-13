use std::cell::Cell;
use crate::constraint_writer::{ConstraintRecoveryMode, ConstraintWriter, ConstraintWriterLookahead};
use crate::constraints::Constraints;
use crate::error::FormatResult;
use crate::error_emitter::{BufferedErrorEmitter, Error};
use self::source_reader::SourceReader;
use crate::util::chars::is_closer_char;
use rustc_span::{BytePos, Pos, Span};
use std::rc::Rc;
use crate::num::HPos;

mod whitespace;
mod source_reader;
pub mod checkpoint;

#[derive(Debug)]
pub struct SourceFormatterLookahead {
    error_buffer: Vec<Error>,
    source_pos: BytePos,
    writer_lookahead: ConstraintWriterLookahead,
}

pub struct SourceFormatter {
    checkpoint_count: Cell<u32>,
    error_emitter: Rc<BufferedErrorEmitter>,
    source: SourceReader,
    out: ConstraintWriter,
    /// The number of spaces for the current level of indentation
    pub indent: Cell<HPos>,
}

macro_rules! delegate_to_constraint_writer {
    ($($(#[$attr:meta])? $vis:vis fn $name:ident $(<$gen:tt>)?(&self $(, $arg:ident: $ty:ty)*) $(-> $ret_ty:ty)? ;)*) => {
        impl SourceFormatter {
            $(
            $(#[$attr])?
            $vis fn $name $(<$gen>)? (&self $(, $arg: $ty)*) $(-> $ret_ty)? {
                self.out.$name($($arg),*)
            })*
        }
    }
}

delegate_to_constraint_writer! {
    pub fn constraints(&self) -> &Constraints;
    pub fn current_max_width(&self) -> HPos;
    #[track_caller]
    pub fn debug_buffer(&self);
    pub fn has_any_constraint_recovery(&self) -> bool;
    pub fn max_recovery_mode(&self) -> ConstraintRecoveryMode;
    pub fn with_constraint_recovery_mode_max<T>(&self, mode: ConstraintRecoveryMode, scope: impl FnOnce() -> T) -> T;
    pub fn with_enforce_max_width<T>(&self, scope: impl FnOnce() -> T) -> T;
    // todo make sure any math using two values of this are guaranteed to be on the same line
    pub fn last_line_len(&self) -> HPos;
    pub fn line(&self) -> u32;
    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T;
}

impl SourceFormatter {
    pub fn new(
        source: Rc<String>,
        constraints: Constraints,
        error_emitter: Rc<BufferedErrorEmitter>,
    ) -> SourceFormatter {
        let capacity = source.len() * 2;
        let out = ConstraintWriter::new(constraints, Rc::clone(&error_emitter), capacity);
        SourceFormatter {
            checkpoint_count: Cell::new(0),
            error_emitter,
            source: SourceReader::new(source),
            out,
            indent: Cell::new(0),
        }
    }

    pub fn finish(self) -> String {
        assert_eq!(self.checkpoint_count.get(), 0);
        self.source.finish();
        self.out.finish()
    }

    pub fn line_col(&self) -> (u32, HPos) {
        (self.out.line(), self.out.last_line_len())
    }

    pub fn source_pos(&self) -> BytePos {
        self.source.pos.get()
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
}
