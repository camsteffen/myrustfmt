use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::list_config::{ParamListConfig, struct_field_list_config};
use crate::ast_formatter::list::{Braces, list};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl AstFormatter {
    pub fn pat(&self, pat: &ast::Pat) -> FormatResult {
        self.pat_tail(pat, Tail::none())
    }

    pub fn pat_tail(&self, pat: &ast::Pat, end: &Tail) -> FormatResult {
        match pat.kind {
            ast::PatKind::Wild => self.out.token("_")?,
            ast::PatKind::Ident(ast::BindingMode(by_ref, mutbl), ident, ref pat) => {
                self.mutability(mutbl)?;
                match by_ref {
                    ast::ByRef::No => {}
                    ast::ByRef::Yes(ref_mutbl) => {
                        self.out.token_space("ref")?;
                        self.mutability(ref_mutbl)?;
                    }
                }
                self.ident(ident)?;
                if let Some(pat) = pat {
                    self.out.space_token_space("@")?;
                    self.pat(pat)?;
                }
                self.tail(end)?;
            }
            ast::PatKind::Struct(ref qself, ref path, ref fields, rest) => {
                self.struct_pat(qself, path, fields, rest, end)?
            }
            ast::PatKind::TupleStruct(ref qself, ref path, ref fields) => {
                self.qpath(qself, path, false)?;
                list(Braces::PARENS, fields, |pat| self.pat(pat))
                    .config(ParamListConfig {
                        single_line_max_contents_width: None,
                    })
                    .tail(end)
                    .format(self)?
            }
            ast::PatKind::Or(ref pats) => {
                self.infix_chain("|", pats, |pat| self.pat(pat), false)?
            }
            ast::PatKind::Path(ref qself, ref path) => self.qpath(qself, path, false)?,
            ast::PatKind::Tuple(ref fields) => list(Braces::PARENS, fields, |pat| self.pat(pat))
                .config(ParamListConfig {
                    single_line_max_contents_width: None,
                })
                .tail(end)
                .format(self)?,
            ast::PatKind::Box(_) => todo!(),
            ast::PatKind::Deref(_) => todo!(),
            ast::PatKind::Ref(ref pat, mutability) => {
                self.out.token("&")?;
                self.mutability(mutability)?;
                self.pat(pat)?;
            }
            ast::PatKind::Lit(_) => todo!(),
            ast::PatKind::Range(_, _, _) => todo!(),
            ast::PatKind::Slice(ref elements) => {
                list(Braces::SQUARE, elements, |pat| self.pat(pat)).format(self)?
            }
            ast::PatKind::Rest => self.out.token("..")?,
            ast::PatKind::Never => todo!(),
            ast::PatKind::Paren(_) => todo!(),
            ast::PatKind::MacCall(_) => todo!(),
            ast::PatKind::Err(_) => todo!(),
        }
        Ok(())
    }

    fn struct_pat(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        fields: &[ast::PatField],
        rest: ast::PatFieldsRest,
        end: &Tail,
    ) -> FormatResult {
        self.qpath(qself, path, false)?;
        self.out.space()?;
        list(Braces::CURLY, fields, |f| self.pat_field(f))
            .config(struct_field_list_config(
                RUSTFMT_CONFIG_DEFAULTS.struct_lit_width,
            ))
            .rest(ListRest::from(rest))
            .tail(end)
            .format(self)
    }

    fn pat_field(&self, pat_field: &ast::PatField) -> FormatResult {
        self.with_attrs(&pat_field.attrs, pat_field.span, || {
            if pat_field.is_shorthand {
                self.pat(&pat_field.pat)?;
            } else {
                self.ident(pat_field.ident)?;
                self.out.token_space(":")?;
                self.pat(&pat_field.pat)?;
            }
            Ok(())
        })
    }
}
