use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::options::ListOptions;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn generic_params(&self, params: &[ast::GenericParam]) -> FormatResult {
        if params.is_empty() {
            if self.out.skip_token_if_present("<")? {
                self.out.skip_token(">")?;
            }
            return Ok(());
        }
        self.list(
            Braces::Angle,
            params,
            Self::generic_param,
            ListOptions::new(),
        )
    }

    // todo breakpoints
    fn generic_param(
        &self,
        param: &ast::GenericParam,
        tail: Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        match param.kind {
            ast::GenericParamKind::Const {
                ref ty,
                ref default,
                ..
            } => {
                self.out.token_space("const")?;
                self.ident(param.ident)?;
                self.out.token_space(":")?;
                self.ty(ty)?;
                let Some(default) = default else {
                    return self.tail(tail);
                };
                self.out.space_token_space("=")?;
                self.expr_tail(&default.value, tail)?;
            }
            ast::GenericParamKind::Lifetime => {
                self.ident(param.ident)?;
                self.generic_bounds_optional(&param.bounds)?;
                self.tail(tail)?
            }
            ast::GenericParamKind::Type { ref default } => {
                self.ident(param.ident)?;
                // todo this on other types too?
                self.generic_bounds_optional(&param.bounds)?;
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
                        self.generic_bounds(&pred.bounds, None)?;
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
