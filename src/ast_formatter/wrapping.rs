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

    pub fn line_break_indent_fallback(
        &self,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        self.fallback(|| {
            self.out.space()?;
            format(false)?;
            Ok(())
        })
        .next(|| {
            self.indented(|| {
                self.out.newline_indent()?;
                format(true)?;
                Ok(())
            })
        })
        .result()
    }

    pub fn line_break_indent_fallback_optional(
        &self,
        apply: bool,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        if apply {
            self.line_break_indent_fallback(format)
        } else {
            format(false)
        }
    }

    pub fn line_break_fallback_with_optional_indent(
        &self,
        should_indent: bool,
        format: impl Fn(/*broken: */ bool) -> FormatResult,
    ) -> FormatResult {
        self.fallback(|| {
            self.out.space()?;
            format(false)?;
            Ok(())
        })
        .next(|| {
            self.indented_optional(should_indent, || {
                self.out.newline_indent()?;
                format(true)?;
                Ok(())
            })
        })
        .result()
    }
}
