use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::StructListConfig;
use crate::source_formatter::FormatResult;

impl<'a> AstFormatter<'a> {
    pub fn pat(&mut self, pat: &ast::Pat) -> FormatResult {
        match pat.kind {
            ast::PatKind::Wild => todo!(),
            ast::PatKind::Ident(mode, ident, ref pat) => self.ident(ident),
            ast::PatKind::Struct(ref a,ref b,ref c,d) => self.struct_(a,b,c,d),
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
    
    fn struct_(&mut self, qself: &Option<P<ast::QSelf>>, path: &ast::Path, fields: &[ast::PatField], rest: ast::PatFieldsRest) -> FormatResult {
        self.qpath(qself, path)?;
        self.out.space()?;
        self.list(fields, Self::pat_field, StructListConfig)?;
        Ok(())
    }
    
    fn pat_field(&mut self, pat_field: &ast::PatField) -> FormatResult {
        // pat_field.attrs;
        self.ident(pat_field.ident)?;
        if !pat_field.is_shorthand {
            self.out.token_expect(":")?;
            self.out.space()?;
            self.pat(&pat_field.pat)?;
        }
        Ok(())
    }
}
