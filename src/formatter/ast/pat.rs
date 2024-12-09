use rustc_ast::ast;

use crate::formatter::{FormatResult, Formatter};

impl<'a> Formatter<'a> {
    pub fn pat(&mut self, pat: &ast::Pat) -> FormatResult {
        match pat.kind {
            ast::PatKind::Wild => todo!(),
            ast::PatKind::Ident(mode, ident, ref pat) => self.ident(ident),
            ast::PatKind::Struct(_, _, _, _) => todo!(),
            ast::PatKind::TupleStruct(_, _, _) => todo!(),
            ast::PatKind::Or(_) => todo!(),
            ast::PatKind::Path(_, _) => todo!(),
            ast::PatKind::Tuple(_) => todo!(),
            ast::PatKind::Box(_) => todo!(),
            ast::PatKind::Deref(_) => todo!(),
            ast::PatKind::Ref(_, _) => todo!(),
            ast::PatKind::Lit(_) => todo!(),
            ast::PatKind::Range(_, _, _) => todo!(),
            ast::PatKind::Slice(_) => todo!(),
            ast::PatKind::Rest => todo!(),
            ast::PatKind::Never => todo!(),
            ast::PatKind::Paren(_) => todo!(),
            ast::PatKind::MacCall(_) => todo!(),
            ast::PatKind::Err(_) => todo!(),
        }
    }
}
