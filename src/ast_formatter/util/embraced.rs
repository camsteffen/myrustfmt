use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

impl AstFormatter {
    // todo "embraced" is a weird name?
    /// Writes a closing brace. Allows for indented comments between braces.
    pub fn embraced_empty_after_opening(&self, closing_brace: &str) -> FormatResult {
        if self.indented(|| self.out.newline_if_comments())? {
            self.out.indent()?;
        }
        self.out.token(closing_brace)?;
        Ok(())
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
            self.out.newline_above()?;
            self.out.indent()?;
            contents()?;
            self.out.newline_below()?;
            Ok(())
        })?;
        self.out.indent()?;
        Ok(())
    }
}
