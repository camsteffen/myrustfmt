use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn line_break_fallback(
        &self,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        self.fallback(|| {
            self.out.space()?;
            format(false)?;
            Ok(())
        })
        .next(|| {
            self.out.newline_indent()?;
            format(true)?;
            Ok(())
        })
        .result()
    }

    pub fn single_line_or_line_break_indent<T>(
        &self,
        format: impl Fn(/*broken: */ bool) -> FormatResult<T>,
    ) -> FormatResult<T> {
        self.fallback(|| {
            self.with_single_line(|| {
                self.out.space()?;
                format(false)
            })
        })
        .next(|| {
            self.indented(|| {
                self.out.newline_indent()?;
                format(true)
            })
        })
        .result()
    }

    pub fn single_line_or_line_break_indent_optional(
        &self,
        apply: bool,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        if !apply {
            return format(false);
        }
        self.single_line_or_line_break_indent(format)
    }

    pub fn line_break_fallback_with_optional_indent(
        &self,
        should_indent: bool,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        self.line_break_fallback(|broken| {
            self.indented_optional(should_indent && broken, || format(broken))
        })
    }
}
