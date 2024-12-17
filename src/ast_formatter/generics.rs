use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{list_overflow_no, AngleBracketedListConfig};
use crate::source_formatter::FormatResult;

use rustc_ast::ast;

impl AstFormatter<'_> {
    pub fn generics(&mut self, generics: &ast::Generics) -> FormatResult {
        if generics.params.is_empty() {
            return Ok(());
        }
        self.list(
            &generics.params,
            Self::generic_param,
            AngleBracketedListConfig,
            list_overflow_no(),
            Tail::NONE,
        )
    }

    fn generic_param(&mut self, param: &ast::GenericParam) -> FormatResult {
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
