use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::error::FormatResult;

use crate::ast_formatter::list::builder::list;
use rustc_ast::ast;

impl AstFormatter {
    pub fn generic_params(&self, params: &[ast::GenericParam]) -> FormatResult {
        if params.is_empty() {
            return Ok(());
        }
        list(Braces::ANGLE, params, |af, p, _lcx| af.generic_param(p)).format(self)
    }

    fn generic_param(&self, param: &ast::GenericParam) -> FormatResult {
        self.ident(param.ident)?;
        self.generic_bounds_optional(&param.bounds)?;
        match param.kind {
            ast::GenericParamKind::Const {
                ref ty,
                ref default,
                ..
            } => {
                self.out.token_space("const")?;
                if let Some(_default) = default {
                    todo!()
                }
                self.ty(ty)?;
            }
            ast::GenericParamKind::Lifetime => {}
            ast::GenericParamKind::Type { ref default } => {
                if let Some(default) = default {
                    self.out.space_token_space("=")?;
                    self.ty(default)?;
                }
            }
        }
        Ok(())
    }

    pub fn where_clause(
        &self,
        where_clause: &ast::WhereClause,
        is_before_body: bool,
    ) -> FormatResult<bool> {
        if where_clause.is_empty() {
            return Ok(false);
        }
        self.out.newline_within_indent()?;
        self.out.token("where")?;
        self.indented(|| {
            for (i, pred) in where_clause.predicates.iter().enumerate() {
                self.out.newline_within_indent()?;
                match &pred.kind {
                    ast::WherePredicateKind::BoundPredicate(pred) => {
                        self.ty(&pred.bounded_ty)?;
                        self.out.token_space(":")?;
                        self.generic_bounds(&pred.bounds)?;
                    }
                    ast::WherePredicateKind::RegionPredicate(_) => todo!(),
                    ast::WherePredicateKind::EqPredicate(_) => todo!(),
                }
                if is_before_body || i < where_clause.predicates.len() - 1 {
                    self.out.token_maybe_missing(",")?;
                }
            }
            Ok(())
        })?;
        if is_before_body {
            self.out.newline_within_indent()?;
        }
        Ok(true)
    }
}
