use crate::ast_formatter::AstFormatter;
use crate::constraints::{VStruct, VStructSet};
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    /// Writes contents between braces with indentation
    pub fn enclosed_contents(&self, scope: impl FnOnce() -> FormatResult) -> FormatResult {
        self.has_vstruct(VStruct::Block, || {
            self.indented(|| {
                self.out.newline(VerticalWhitespaceMode::Top)?;
                self.out.indent();
                self.constraints()
                    .disallowed_vstructs
                    .with_replaced(VStructSet::new(), scope)?;
                self.out.newline(VerticalWhitespaceMode::Bottom)?;
                Ok(())
            })?;
            self.out.indent();
            Ok(())
        })
    }

    /// Normally nothing, but allows for comments inside like:
    /// (
    ///     // comment
    /// )
    pub fn enclosed_empty_contents(&self) -> FormatResult {
        let first_line = self.out.line();
        self.indented(|| self.out.comments(VerticalWhitespaceMode::Break))?;
        let multi_line = self.out.line() != first_line;
        if multi_line {
            self.out.indent();
        }
        Ok(())
    }
}
