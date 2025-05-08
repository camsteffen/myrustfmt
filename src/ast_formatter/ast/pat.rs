use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::options::ListOptions;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl AstFormatter {
    pub fn pat(&self, pat: &ast::Pat) -> FormatResult {
        self.pat_tail(pat, &None)
    }

    pub fn pat_tail(&self, pat: &ast::Pat, tail: &Tail) -> FormatResult {
        // todo is this right?
        self.out.skip_token_if_present("|")?;

        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();

        match pat.kind {
            ast::PatKind::Expr(ref expr) => self.expr_tail(expr, take_tail())?,
            ast::PatKind::Guard(ref _pat, ref _cond) => todo!(),
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
            }
            ast::PatKind::Struct(ref qself, ref path, ref fields, rest) => {
                self.struct_pat(qself, path, fields, rest, take_tail())?
            }
            ast::PatKind::TupleStruct(ref qself, ref path, ref fields) => {
                self.qpath(qself, path, false)?;
                self.list(
                    Braces::Parens,
                    fields,
                    Self::pat_list_item,
                    ListOptions::new().tail(take_tail()),
                )?;
            }
            ast::PatKind::Or(ref pats) => {
                self.simple_infix_chain("|", pats, |pat| self.pat(pat), false, take_tail())?
            }
            ast::PatKind::Path(ref qself, ref path) => self.qpath(qself, path, false)?,
            ast::PatKind::Tuple(ref fields) => self.list(
                Braces::Parens,
                fields,
                Self::pat_list_item,
                ListOptions::new().tail(take_tail()),
            )?,
            ast::PatKind::Box(_) => todo!(),
            ast::PatKind::Deref(_) => todo!(),
            ast::PatKind::Ref(ref pat, mutability) => {
                self.out.token("&")?;
                self.mutability(mutability)?;
                self.pat(pat)?;
            }
            ast::PatKind::Range(ref start, ref end, ref end_kind) => {
                let sigil = match end_kind.node {
                    ast::RangeEnd::Excluded => "..",
                    ast::RangeEnd::Included(ast::RangeSyntax::DotDotDot) => "...",
                    ast::RangeEnd::Included(ast::RangeSyntax::DotDotEq) => "..=",
                };
                self.range(start.as_deref(), sigil, end.as_deref(), take_tail())?;
            }
            ast::PatKind::Slice(ref elements) => self.list(
                Braces::Square,
                elements,
                Self::pat_list_item,
                ListOptions::new().tail(take_tail()),
            )?,
            ast::PatKind::Rest => self.out.token("..")?,
            ast::PatKind::Never => todo!(),
            ast::PatKind::Paren(_) => todo!(),
            ast::PatKind::MacCall(ref mac_call) => self.mac_call(mac_call)?,
            ast::PatKind::Err(_) => todo!(),
        }

        if let Some(tail) = tail {
            self.tail(tail)?;
        }

        Ok(())
    }

    fn pat_list_item(&self, pat: &P<ast::Pat>, tail: &Tail, _lcx: ListItemContext) -> FormatResult {
        self.pat_tail(pat, tail)
    }

    fn struct_pat(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        fields: &[ast::PatField],
        rest: ast::PatFieldsRest,
        tail: &Tail,
    ) -> FormatResult {
        self.qpath(qself, path, false)?;
        self.out.space()?;
        self.list(
            Braces::Curly,
            fields,
            Self::pat_field,
            ListOptions::new()
                .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
                .rest(ListRest::from_pat_fields_rest(rest))
                .tail(tail),
        )
    }

    fn pat_field(
        &self,
        pat_field: &ast::PatField,
        tail: &Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        self.with_attrs_tail(&pat_field.attrs, pat_field.span, tail, || {
            if !pat_field.is_shorthand {
                self.ident(pat_field.ident)?;
                self.out.token_space(":")?;
            }
            self.pat_tail(&pat_field.pat, tail)?;
            Ok(())
        })
    }
}
