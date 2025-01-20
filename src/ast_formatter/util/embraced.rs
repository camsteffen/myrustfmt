use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

impl AstFormatter {
    /// Writes a closing brace. Allows for indented comments between braces.
    pub fn embraced_empty_after_opening(&self, closing_brace: &str) -> FormatResult {
        self.fallback(|| self.with_single_line(|| self.out.token(closing_brace)))
            .next(|| {
                self.indented(|| self.out.newline_split())?;
                self.out.token(closing_brace)?;
                Ok(())
            })
            .result()
    }

    pub fn embraced_after_opening(
        &self,
        closing_brace: &str,
        contents: impl Fn() -> FormatResult,
    ) -> FormatResult {
        self.embraced_inside(contents)?;
        self.out.token(closing_brace)?;
        Ok(())
    }

    /// Writes contents between braces with indentation
    pub fn embraced_inside(&self, contents: impl FnOnce() -> FormatResult) -> FormatResult {
        self.indented(|| {
            self.out.newline_leading_indent()?;
            contents()?;
            self.out.newline_trailing()?;
            Ok(())
        })?;
        self.out.indent()?;
        Ok(())
    }
}
