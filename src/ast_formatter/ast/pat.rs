use crate::ast_formatter::brackets::Brackets;
use rustc_ast::ast;
use rustc_ast::ptr::P;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ListItemContext;
use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies,
};
use crate::ast_formatter::tail::Tail;
use crate::error::{FormatErrorKind, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl AstFormatter {
    pub fn pat(&self, pat: &ast::Pat) -> FormatResult {
        self.pat_tail(pat, None)
    }

    pub fn pat_tail(&self, pat: &ast::Pat, tail: Tail) -> FormatResult {
        // todo is this right?
        self.out.token_skip_if_present("|")?;

        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();

        match pat.kind {
            ast::PatKind::Expr(ref expr) => self.expr_tail(expr, take_tail())?,
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
            ast::PatKind::MacCall(ref mac_call) => self.macro_call(mac_call, take_tail())?,
            ast::PatKind::Or(ref pats) => {
                self.simple_infix_chain("|", pats, |pat| self.pat(pat), false, take_tail())?
            }
            ast::PatKind::Paren(ref inner) => {
                // todo breakpoint
                self.out.token("(")?;
                self.pat(inner)?;
                self.out.token(")")?;
            }
            ast::PatKind::Path(ref qself, ref path) => self.qpath(qself, path, false, take_tail())?,
            ast::PatKind::Range(ref start, ref end, ref end_kind) => {
                let sigil = match end_kind.node {
                    ast::RangeEnd::Excluded => "..",
                    ast::RangeEnd::Included(ast::RangeSyntax::DotDotDot) => "...",
                    ast::RangeEnd::Included(ast::RangeSyntax::DotDotEq) => "..=",
                };
                self.range(start.as_deref(), sigil, end.as_deref(), take_tail())?;
            }
            ast::PatKind::Ref(ref pat, mutability) => {
                self.out.token("&")?;
                self.mutability(mutability)?;
                self.pat(pat)?;
            }
            ast::PatKind::Rest => self.out.token("..")?,
            ast::PatKind::Slice(ref elements) => self.list(
                Brackets::Square,
                elements,
                Self::pat_list_item,
                ListOptions {
                    tail: take_tail(),
                    ..
                },
            )?,
            ast::PatKind::Struct(ref qself, ref path, ref fields, rest) => {
                self.struct_pat(qself, path, fields, rest, take_tail())?
            }
            ast::PatKind::Tuple(ref fields) => self.list(
                Brackets::Parens,
                fields,
                Self::pat_list_item,
                ListOptions {
                    tail: take_tail(),
                    ..
                },
            )?,
            ast::PatKind::TupleStruct(ref qself, ref path, ref fields) => {
                // todo tail?
                self.qpath(qself, path, false, None)?;
                self.list(
                    Brackets::Parens,
                    fields,
                    Self::pat_list_item,
                    ListOptions {
                        tail: take_tail(),
                        ..
                    },
                )?;
            }
            ast::PatKind::Wild => self.out.token("_")?,
            ast::PatKind::Box(_)
            | ast::PatKind::Deref(_)
            | ast::PatKind::Guard(..)
            | ast::PatKind::Never => return Err(FormatErrorKind::UnsupportedSyntax.into()),
            ast::PatKind::Err(_) | ast::PatKind::Missing => panic!("unexpected PatKind"),
        }

        if let Some(tail) = tail {
            self.tail(tail)?;
        }

        Ok(())
    }

    fn pat_list_item(&self, pat: &P<ast::Pat>, tail: Tail, _lcx: ListItemContext) -> FormatResult {
        self.pat_tail(pat, tail)
    }

    fn struct_pat(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        fields: &[ast::PatField],
        rest: ast::PatFieldsRest,
        tail: Tail,
    ) -> FormatResult {
        // todo tail?
        self.qpath(qself, path, false, None)?;
        self.out.space()?;
        self.list(
            Brackets::Curly,
            fields,
            Self::pat_field,
            ListOptions {
                is_struct: true,
                rest: ListRest::from_pat_fields_rest(rest),
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
                    horizontal: HorizontalListStrategy {
                        contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width),
                        ..
                    },
                    ..
                }),
                tail,
                ..
            },
        )?;
        Ok(())
    }

    fn pat_field(
        &self,
        pat_field: &ast::PatField,
        tail: Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        self.with_attrs_tail(&pat_field.attrs, pat_field.span.into(), tail, || {
            if !pat_field.is_shorthand {
                self.ident(pat_field.ident)?;
                self.out.token_space(":")?;
            }
            self.pat_tail(&pat_field.pat, tail)?;
            Ok(())
        })
    }
}
