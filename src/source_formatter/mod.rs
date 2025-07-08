pub mod checkpoint;
mod source_reader;
mod whitespace_and_comments;

use self::source_reader::SourceReader;
use crate::constraint_writer::ConstraintWriter;
use crate::constraints::Constraints;
use crate::error::FormatResult;
use crate::error_emitter::BufferedErrorEmitter;
use crate::num::{HSize, VSize};
use crate::span::Span;
use crate::util::chars::is_closer_char;
use rustc_span::SourceFile;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

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
        self.source_reader.eat_token(token);
        self.out.token(token)?;
        Ok(())
    }

    /// Inserts a token without expecting it in the source
    pub fn token_insert(&self, token: &'static str) -> FormatResult {
        self.out.token(token)
    }

    /// Replaces the next token
    pub fn token_replace(&self, token: &'static str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.eat_next_token();
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
        self.token_skip_if_present(token)?;
        self.token_insert(token)?;
        Ok(())
    }

    /// Copy a token from source. Must be known to not have newlines.
    pub fn token_from_source(&self, span: Span) -> FormatResult {
        self.horizontal_whitespace()?;
        let token = self.source_reader.eat_span(span);
        self.out.token(token)?;
        Ok(())
    }

    pub fn token_skip(&self, token: &'static str) -> FormatResult {
        self.horizontal_whitespace()?;
        self.source_reader.eat_token(token);
        Ok(())
    }

    pub fn token_skip_if_present(&self, token: &str) -> FormatResult<bool> {
        // todo is this checkpoint avoidable?
        let checkpoint = self.checkpoint();
        let found = self.horizontal_whitespace().is_ok() && self.source_reader.try_eat_token(token);
        if !found {
            self.restore_checkpoint(&checkpoint);
        }
        Ok(found)
    }

    pub fn copy_span(&self, span: Span) -> FormatResult {
        // If there is any whitespace next, if it is part of the span, we want to copy it with the
        // rest of the span. If it is not part of the span, we should consume it as normal
        // whitespace.
        if self.source_reader.pos.get() < span.lo {
            self.horizontal_whitespace()?;
        }
        let segment = self.source_reader.eat_span(span);
        self.out.write_str(segment)?;
        Ok(())
    }

    /// Copies a segment from source without enforcing constraints
    fn copy_unchecked(&self, len: u32) {
        let segment = self.source_reader.eat_len(len);
        self.out.write_str_unchecked(segment);
    }

    pub fn last_line_is_closers(&self) -> bool {
        self.with_last_line(|line| {
            let after_indent = &line[self.total_indent.get().into()..];
            after_indent.bytes().all(is_closer_char)
        })
    }
}
