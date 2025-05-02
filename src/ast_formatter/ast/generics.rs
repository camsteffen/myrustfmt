use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::error::FormatResult;

use rustc_ast::ast;
use crate::ast_formatter::list::options::list_opt;
use crate::ast_formatter::tail::Tail;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn generic_params(&self, params: &[ast::GenericParam]) -> FormatResult {
        if params.is_empty() {
            if self.out.skip_token_if_present("<")? {
                self.out.skip_token(">")?;
            }
            return Ok(());
        }
        self.list(Braces::Angle, params, Self::generic_param, list_opt())
    }

    fn generic_param(
        &self,
        param: &ast::GenericParam,
        tail: &Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
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
                self.ty_tail(ty, tail)?;
            }
            ast::GenericParamKind::Lifetime => self.tail(tail)?,
            ast::GenericParamKind::Type { ref default } => {
                if let Some(default) = default {
                    self.out.space_token_space("=")?;
                    self.ty_tail(default, tail)?;
                } else {
                    self.tail(tail)?;
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
        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
        self.out.token("where")?;
        self.indented(|| {
            for (i, pred) in where_clause.predicates.iter().enumerate() {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                match &pred.kind {
                    ast::WherePredicateKind::BoundPredicate(pred) => {
                        self.ty(&pred.bounded_ty)?;
                        self.out.token_space(":")?;
                        self.generic_bounds(&pred.bounds, Tail::none())?;
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
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
        }
        Ok(true)
    }
}
