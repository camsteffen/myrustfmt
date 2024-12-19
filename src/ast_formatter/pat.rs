use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{StructFieldListConfig, list, param_list_config};
use crate::source_formatter::FormatResult;

impl<'a> AstFormatter {
    pub fn pat(&mut self, pat: &ast::Pat) -> FormatResult {
        self.pat_end(pat, Tail::NONE)
    }

    pub fn pat_end(&mut self, pat: &ast::Pat, end: Tail<'_>) -> FormatResult {
        match pat.kind {
            ast::PatKind::Wild => todo!(),
            ast::PatKind::Ident(ast::BindingMode(by_ref, mutbl), ident, ref pat) => {
                self.mutability(mutbl)?;
                match by_ref {
                    ast::ByRef::No => {}
                    ast::ByRef::Yes(ref_mutbl) => {
                        self.out.token_expect("ref")?;
                        self.out.space()?;
                        self.mutability(ref_mutbl)?;
                    }
                }
                self.ident(ident)?;
                self.tail(end)
            }
            ast::PatKind::Struct(ref qself, ref path, ref fields, rest) => {
                self.struct_pat(qself, path, fields, rest, end)
            }
            ast::PatKind::TupleStruct(ref qself, ref path, ref fields) => {
                self.qpath(qself, path)?;
                list(fields, |this, pat| this.pat(pat), param_list_config(None))
                    .tail(end)
                    .format(self)
            }
            ast::PatKind::Or(_) => todo!(),
            ast::PatKind::Path(_, _) => todo!(),
            ast::PatKind::Tuple(ref fields) => {
                list(fields, |this, pat| this.pat(pat), param_list_config(None))
                    .tail(end)
                    .format(self)
            }
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

    fn struct_pat(
        &mut self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        fields: &[ast::PatField],
        rest: ast::PatFieldsRest,
        end: Tail<'_>,
    ) -> FormatResult {
        self.qpath(qself, path)?;
        self.out.space()?;
        list(fields, Self::pat_field, StructFieldListConfig)
            .tail(end)
            .format(self)
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
