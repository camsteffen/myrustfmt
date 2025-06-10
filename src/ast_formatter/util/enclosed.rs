use crate::ast_formatter::AstFormatter;
use crate::constraints::VStructSet;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn enclosed_empty_after_opening(&self, closing_brace: &'static str) -> FormatResult {
        let first_line = self.out.line();
        self.indented(|| self.out.comments(VerticalWhitespaceMode::Break))?;
        let multi_line = self.out.line() != first_line;
        if multi_line {
            self.out.indent();
        }
        self.out.token(closing_brace)?;
        Ok(())
    }

    pub fn enclosed_after_opening(
        &self,
        close_brace: &'static str,
        contents: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.enclosed_contents(contents)?;
        self.out.token(close_brace)?;
        Ok(())
    }

    /// Writes contents between braces with indentation
    pub fn enclosed_contents(&self, contents: impl FnOnce() -> FormatResult) -> FormatResult {
        self.indented(|| {
            self.out.newline(VerticalWhitespaceMode::Top)?;
            self.out.indent();
            self.constraints().disallowed_vstructs.with_replaced(
                VStructSet::new(),
                contents,
            )?;
            self.out.newline(VerticalWhitespaceMode::Bottom)?;
            Ok(())
        })?;
        self.out.indent();
        Ok(())
    }
}
