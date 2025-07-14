use crate::ast_formatter::util::indent::IndentGuard;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::{FormatErrorKind, FormatResult};
use crate::util::drop::Guard;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    /// If the current position is farther right compared to the position if wrap-indented, then
    /// width is recoverable.
    pub fn could_wrap_indent_guard(&self) -> impl Guard {
        (self.out.col() > self.out.total_indent.get() + INDENT_WIDTH)
            .then(|| self.recover_width_guard())
    }

    // todo test
    pub fn space_or_wrap_then(&self, then: impl Fn() -> FormatResult) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        if self.out.space_allow_newlines()? {
            return then();
        }
        self.backtrack()
            .next(|_| {
                let _guard = self.recover_width_guard();
                then()?;
                Ok(())
            })
            .next(|_| {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                then()?;
                Ok(())
            })
            .result_with_checkpoint(&checkpoint)?;
        Ok(())
    }

    pub fn space_or_wrap_indent_then(
        &self,
        then: impl Fn() -> FormatResult,
    ) -> FormatResult<Option<IndentGuard<'_>>> {
        let checkpoint = self.out.checkpoint();
        let first_line = self.out.line();
        let indent_guard = self.begin_indent();
        if self.out.space_allow_newlines()? {
            // wrap forced by comments
            then()?;
            return Ok(Some(indent_guard));
        }
        // N.B. observe indent before dropping indent guard
        let col_if_wrap_indent = self.out.total_indent.get();
        drop(indent_guard);
        if self.out.col() <= col_if_wrap_indent {
            // wrapping does not give more width
            then()?;
            return Ok(None);
        }
        let result = {
            let _guard = self.recover_width_guard();
            then()
        };
        if self.out.line() != first_line {
            result?;
            return Ok(Some(self.begin_indent()));
        }
        match result {
            Err(e) if matches!(e.kind, FormatErrorKind::WidthLimitExceeded) => {
                self.out.restore_checkpoint(&checkpoint);
                let indent_guard = self.begin_indent();
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                then()?;
                Ok(Some(indent_guard))
            }
            Err(e) => Err(e),
            Ok(()) => Ok(None),
        }
    }
}
