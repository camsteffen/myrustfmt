pub mod checkpoint;
mod source_reader;
mod whitespace;

use self::source_reader::SourceReader;
use crate::constraint_writer::ConstraintWriter;
use crate::constraint_writer::checkpoint::ConstraintWriterLookahead;
use crate::constraints::Constraints;
use crate::error::FormatResult;
use crate::error_emitter::{BufferedErrorEmitter, Error};
use crate::num::{HSize, VSize};
use crate::util::chars::is_closer_char;
use rustc_span::{BytePos, Pos, SourceFile, Span};
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub struct Lookahead {
    error_buffer: Vec<Error>,
    source_pos: BytePos,
    writer_lookahead: ConstraintWriterLookahead,
}

pub struct SourceFormatter {
    // checkpoint_count: Cell<u32>,
    error_emitter: Rc<BufferedErrorEmitter>,
    // This should be encapsulated, but we break that rule sometimes
    pub source_reader: SourceReader,
    out: ConstraintWriter,
    /// The number of spaces for the current level of indentation
    pub total_indent: Cell<HSize>,
}

macro_rules! delegate_to_constraint_writer {
    ($($(#[$attr:meta])* $vis:vis fn $name:ident $(<$gen:tt>)?(&self $(, $arg:ident: $ty:ty)*) $(-> $ret_ty:ty)? ;)*) => {
        impl SourceFormatter {
            $(
            $(#[$attr])*
            $vis fn $name $(<$gen>)? (&self $(, $arg: $ty)*) $(-> $ret_ty)? {
                self.out.$name($($arg),*)
            })*
        }
    }
}

delegate_to_constraint_writer! {
    pub fn constraints(&self) -> &Constraints;
    pub fn with_recover_width<T>(&self, scope: impl FnOnce() -> T) -> T;
    pub fn line(&self) -> VSize;
    pub fn col(&self) -> HSize;
    pub fn line_col(&self) -> (VSize, HSize);
    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T;

    #[allow(unused)]
    #[track_caller]
    pub fn debug_buffer(&self);
    #[allow(unused)]
    pub fn with_taken_buffer(&self, f: impl FnOnce(&mut String));
}

impl SourceFormatter {
    pub fn new(
        path: Option<PathBuf>,
        source_file: Arc<SourceFile>,
        error_emitter: Rc<BufferedErrorEmitter>,
        max_width: HSize,
    ) -> SourceFormatter {
        let source_reader = SourceReader::new(path, source_file);
        let capacity = source_reader.source().len() * 2;
        let out = ConstraintWriter::new(max_width, Rc::clone(&error_emitter), capacity);
        SourceFormatter {
            error_emitter,
            source_reader,
            out,
            total_indent: Cell::new(0),
        }
    }

    pub fn finish(self) -> String {
        self.source_reader.finish();
        self.out.finish()
    }

    pub fn skip_token(&self, token: &'static str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.eat(token);
        Ok(())
    }

    pub fn skip_token_if_present(&self, token: &str) -> FormatResult<bool> {
        // todo is this checkpoint avoidable?
        let checkpoint = self.checkpoint();
        let ws_result = self.horizontal_whitespace();
        if self.source_reader.remaining().starts_with(token) {
            ws_result?;
            self.source_reader.advance(token.len().try_into().unwrap());
            Ok(true)
        } else {
            self.restore_checkpoint(&checkpoint);
            Ok(false)
        }
    }

    pub fn copy_next_token(&self) -> FormatResult {
        self.horizontal_whitespace()?;
        let token = self.source_reader.eat_next_token();
        self.out.token(token)?;
        Ok(())
    }

    /// Write a token, asserting it is next in source.
    ///
    /// N.B. a token should not contain whitespace
    /// N.B. a token is indivisible (e.g. "::<" is two tokens since you can write it as "::  <")
    pub fn token(&self, token: &'static str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.eat(token);
        self.out.token(token)?;
        Ok(())
    }

    /// Inserts a token without expecting it in the source
    pub fn token_insert(&self, token: &'static str) -> FormatResult {
        self.out.token(token)
    }

    pub fn token_space(&self, token: &'static str) -> FormatResult {
        self.token(token)?;
        self.space()?;
        Ok(())
    }

    pub fn space_token_space(&self, token: &'static str) -> FormatResult {
        self.space()?;
        self.token(token)?;
        self.space()?;
        Ok(())
    }

    pub fn space_token(&self, token: &'static str) -> FormatResult {
        self.space()?;
        self.token(token)?;
        Ok(())
    }

    /// Write a token that might be missing from source
    pub fn token_maybe_missing(&self, token: &'static str) -> FormatResult {
        self.skip_token_if_present(token)?;
        self.token_insert(token)?;
        Ok(())
    }

    /// Copy a token from source
    pub fn token_from_source(&self, span: Span) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.expect_pos(span.lo());
        let token = self.source_reader.get_span(span);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn copy(&self, len: u32) -> FormatResult {
        let segment = &self.source_reader.remaining()[..len.try_into().unwrap()];
        self.out.write_possibly_multiline(segment)?;
        self.source_reader.advance(len);
        Ok(())
    }

    pub fn copy_span(&self, span: Span) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.expect_pos(span.lo());
        self.copy(span.hi().to_u32() - span.lo().to_u32())?;
        Ok(())
    }

    /** Write a token assuming it is next in source */
    fn token_unchecked(&self, token: &str) -> FormatResult {
        self.out.token(token)?;
        self.source_reader.advance(token.len().try_into().unwrap());
        Ok(())
    }

    pub fn last_line_is_closers(&self) -> bool {
        self.with_last_line(|line| {
            let after_indent = &line[self.total_indent.get().try_into().unwrap()..];
            after_indent.chars().all(is_closer_char)
        })
    }
}
