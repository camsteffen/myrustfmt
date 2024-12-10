use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter<'_> {
    pub fn qpath(&mut self, qself: &Option<P<ast::QSelf>>, path: &ast::Path) -> FormatResult {
        if let Some(qself) = qself.as_deref() {
            todo!();
        }
        self.path(path)?;
        Ok(())
    }

    fn path(&mut self, path: &ast::Path) -> FormatResult {
        for segment in &path.segments {
            self.path_segment(segment)?;
        }
        Ok(())
    }

    fn path_segment(&mut self, segment: &ast::PathSegment) -> FormatResult {
        self.ident(segment.ident)?;
        if let Some(args) = &segment.args {
            todo!();
        }
        Ok(())
    }
}