use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{AngleBracketedListConfig, list};
use crate::source_formatter::FormatResult;

use rustc_ast::ast;

impl AstFormatter {
    pub fn generics(&self, generics: &ast::Generics) -> FormatResult {
        if generics.params.is_empty() {
            return Ok(());
        }
        list(
            &generics.params,
            |p| self.generic_param(p),
            AngleBracketedListConfig,
        )
        .format(self)
    }

    fn generic_param(&self, param: &ast::GenericParam) -> FormatResult {
        self.ident(param.ident)?;
        self.generic_bounds(&param.bounds)?;
        match param.kind {
            ast::GenericParamKind::Const {
                ref ty,
                kw_span,
                ref default,
            } => {
                self.out.token_at_space("const", kw_span.lo())?;
                if let Some(default) = default {
                    todo!()
                }
                self.ty(ty)?;
            }
            ast::GenericParamKind::Lifetime => {}
            ast::GenericParamKind::Type { ref default } => {
                if let Some(default) = default {
                    todo!()
                }
            }
        }
        Ok(())
    }
}
