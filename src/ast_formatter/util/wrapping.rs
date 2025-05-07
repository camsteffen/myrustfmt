use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::indent::IndentGuard;
use crate::error::{ConstraintErrorKind, FormatResult};
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn space_or_wrap_then(&self, then: impl Fn() -> FormatResult) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        let first_line = self.out.line();
        self.out.space_or_break()?;
        let result = self.out.with_enforce_max_width(&then);
        if self.out.line() == first_line && result.is_err() {
            self.out.restore_checkpoint(&checkpoint);
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            then()?;
        } else {
            result?;
        }
        Ok(())
    }

    pub fn space_or_wrap_indent_then(
        &self,
        then: impl Fn() -> FormatResult,
    ) -> FormatResult<Option<IndentGuard>> {
        let checkpoint = self.out.checkpoint();
        let first_line = self.out.line();
        let indent_guard = self.begin_indent();
        self.out.space_or_break()?;
        if self.out.line() != first_line {
            then()?;
            return Ok(Some(indent_guard));
        }
        let start_if_wrap = self.out.total_indent.get();
        drop(indent_guard);
        let wrap_has_more_width = self.out.col() > start_if_wrap;
        let result = self.out.with_enforce_max_width(&then);
        if self.out.line() != first_line {
            result?;
            return Ok(Some(self.begin_indent()));
        }
        match result {
            Err(e)
                if wrap_has_more_width && matches!(e.kind, ConstraintErrorKind::WidthLimitExceeded) =>
            {
                self.out.restore_checkpoint(&checkpoint);
                let indent_guard = self.begin_indent();
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                then()?;
                return Ok(Some(indent_guard));
            }
            _ => {}
        }
        result?;
        Ok(None)
    }
}
