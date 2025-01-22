use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, list};
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

    pub fn where_clause(&self, where_clause: &ast::WhereClause) -> FormatResult {
        if where_clause.is_empty() {
            return Ok(());
        }
        self.out.newline_within_indent()?;
        self.out.token("where")?;
        self.indented(|| {
            for pred in &where_clause.predicates {
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
                self.out.token(",")?;
            }
            Ok(())
        })?;
        self.out.newline_within_indent()?;
        Ok(())
    }
}
