use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{list, Braces};
use crate::error::FormatResult;

use rustc_ast::ast;

impl AstFormatter {
    pub fn generic_params(&self, params: &[ast::GenericParam]) -> FormatResult {
        if params.is_empty() {
            return Ok(());
        }
        list(Braces::ANGLE, params, |p| self.generic_param(p)).format(self)
    }

    fn generic_param(&self, param: &ast::GenericParam) -> FormatResult {
        self.ident(param.ident)?;
        if !param.bounds.is_empty() {
            self.out.token_expect(":")?;
            self.out.space()?;
            self.generic_bounds(&param.bounds)?;
        }
        match param.kind {
            ast::GenericParamKind::Const {
                ref ty,
                kw_span,
                ref default,
            } => {
                self.out.token_at_space("const", kw_span.lo())?;
                if let Some(_default) = default {
                    todo!()
                }
                self.ty(ty)?;
            }
            ast::GenericParamKind::Lifetime => {}
            ast::GenericParamKind::Type { ref default } => {
                if let Some(default) = default {
                    self.out.token_expect("=")?;
                    self.ty(default)?;
                }
            }
        }
        Ok(())
    }

    pub fn where_clause(&self, where_clause: &ast::WhereClause) -> FormatResult {
        if where_clause.is_empty() {
            return Ok(());
        }
        self.out.newline_indent()?;
        self.out.token_expect("where")?;
        self.indented(|| {
            where_clause.predicates.iter().try_for_each(|pred| {
                self.out.newline_indent()?;
                match &pred.kind {
                    ast::WherePredicateKind::BoundPredicate(pred) => {
                        self.ty(&pred.bounded_ty)?;
                        self.out.token_expect(":")?;
                        self.out.space()?;
                        self.generic_bounds(&pred.bounds)?;
                    }
                    ast::WherePredicateKind::RegionPredicate(_) => todo!(),
                    ast::WherePredicateKind::EqPredicate(_) => todo!(),
                }
                self.out.token_expect(",")?;
                Ok(())
            })
        })?;
        self.out.newline_indent()?;
        Ok(())
    }
}
