use crate::ast_formatter::util::indent::IndentGuard;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::{FormatErrorKind, FormatResult};
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    /// If the current position is farther right compared to the position if wrap-indented, then
    /// width is recoverable for the given scope.
    pub fn could_wrap_indent<T>(&self, scope: impl Fn() -> FormatResult<T>) -> FormatResult<T> {
        if self.out.col() <= self.out.total_indent.get() + INDENT_WIDTH {
            scope()
        } else {
            self.out.with_recover_width(scope)
        }
    }

    pub fn space_could_wrap_indent(&self, scope: impl Fn() -> FormatResult) -> FormatResult {
        self.out.space()?;
        self.could_wrap_indent(scope)?;
        Ok(())
    }

    // todo should these functions use backtrack?
    pub fn space_or_wrap_then(&self, then: impl Fn() -> FormatResult) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        let first_line = self.out.line();
        self.out.space_allow_comments()?;
        let result = self.out.with_recover_width(&then);
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
        if self.out.space_allow_comments()? {
            // wrap forced by comments
            then()?;
            return Ok(Some(indent_guard));
        }
        // N.B. observe indent before closing indent guard
        let col_if_wrap_indent = self.out.total_indent.get();
        indent_guard.close();
        if self.out.col() <= col_if_wrap_indent {
            // wrapping does not give more width
            then()?;
            return Ok(None);
        }
        let result = self.out.with_recover_width(&then);
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
