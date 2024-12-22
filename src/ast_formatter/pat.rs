use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::config::{ParamListConfig, struct_field_list_config};
use crate::ast_formatter::list::{Braces, ListRest, list};
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl<'a> AstFormatter {
    pub fn pat(&self, pat: &ast::Pat) -> FormatResult {
        self.pat_tail(pat, Tail::NONE)
    }

    pub fn pat_tail(&self, pat: &ast::Pat, end: Tail<'_>) -> FormatResult {
        match pat.kind {
            ast::PatKind::Wild => self.out.token_expect("_"),
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
                if let Some(pat) = pat {
                    self.out.space()?;
                    self.out.token_expect("@")?;
                    self.out.space()?;
                    self.pat(pat)?;
                }
                self.tail(end)
            }
            ast::PatKind::Struct(ref qself, ref path, ref fields, rest) => {
                self.struct_pat(qself, path, fields, rest, end)
            }
            ast::PatKind::TupleStruct(ref qself, ref path, ref fields) => {
                self.qpath(qself, path, false)?;
                list(Braces::PARENS, fields, |pat| self.pat(pat))
                    .config(&ParamListConfig {
                        single_line_max_contents_width: None,
                    })
                    .tail(end)
                    .format(self)
            }
            ast::PatKind::Or(ref pats) => {
                let (first, rest) = pats.split_first().unwrap();
                self.pat(first)?;
                for pat in rest {
                    self.out.space()?;
                    self.out.token_expect("|")?;
                    self.out.space()?;
                    self.pat(pat)?;
                }
                Ok(())
            }
            ast::PatKind::Path(ref qself, ref path) => self.qpath(qself, path, false),
            ast::PatKind::Tuple(ref fields) => list(Braces::PARENS, fields, |pat| self.pat(pat))
                .config(&ParamListConfig {
                    single_line_max_contents_width: None,
                })
                .tail(end)
                .format(self),
            ast::PatKind::Box(_) => todo!(),
            ast::PatKind::Deref(_) => todo!(),
            ast::PatKind::Ref(_, _) => todo!(),
            ast::PatKind::Lit(_) => todo!(),
            ast::PatKind::Range(_, _, _) => todo!(),
            ast::PatKind::Slice(ref elements) => {
                list(Braces::SQUARE, elements, |pat| self.pat(pat)).format(self)
            }
            ast::PatKind::Rest => self.out.token_expect(".."),
            ast::PatKind::Never => todo!(),
            ast::PatKind::Paren(_) => todo!(),
            ast::PatKind::MacCall(_) => todo!(),
            ast::PatKind::Err(_) => todo!(),
        }
    }

    fn struct_pat(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        fields: &[ast::PatField],
        rest: ast::PatFieldsRest,
        end: Tail<'_>,
    ) -> FormatResult {
        self.qpath(qself, path, false)?;
        self.out.space()?;
        let single_line_block =
            self.config().rustfmt_quirks && matches!(rest, ast::PatFieldsRest::Rest);
        list(Braces::CURLY, fields, |f| self.pat_field(f))
            .config(&struct_field_list_config(
                single_line_block,
                RUSTFMT_CONFIG_DEFAULTS.struct_lit_width,
            ))
            .rest(ListRest::from(rest))
            .tail(end)
            .format(self)
    }

    fn pat_field(&self, pat_field: &ast::PatField) -> FormatResult {
        self.attrs(&pat_field.attrs)?;
        if pat_field.is_shorthand {
            self.pat(&pat_field.pat)?;
        } else {
            self.ident(pat_field.ident)?;
            self.out.token_expect(":")?;
            self.out.space()?;
            self.pat(&pat_field.pat)?;
        }
        Ok(())
    }
}
