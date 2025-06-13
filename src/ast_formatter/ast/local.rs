use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use crate::num::{HSize, VSize};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn local(&self, local: &ast::Local) -> FormatResult {
        self.with_attrs(&local.attrs, local.span, || self.local_after_attrs(local))
    }

    fn local_after_attrs(&self, local: &ast::Local) -> FormatResult {
        let (first_line, start_col) = self.out.line_col();
        self.out.token_space("let")?;
        self.pat_tail(
            &local.pat,
            Some(
                &self.tail_fn(|af| af.local_after_pat(local, first_line, start_col)),
            ),
        )?;
        Ok(())
    }

    fn local_after_pat(
        &self,
        local: &ast::Local,
        first_line: VSize,
        start_col: HSize,
    ) -> FormatResult {
        let ast::Local { kind, ty, .. } = local;
        let init_else = |af: &AstFormatter| {
            let Some((init, else_)) = kind.init_else_opt() else {
                return af.out.token(";");
            };
            af.assign_expr(init, else_.is_none().then(|| af.tail_token(";")).as_ref())?;
            let Some(else_) = else_ else { return Ok(()) };
            let is_single_line_init = af.out.line() == first_line;
            af.local_else(else_, is_single_line_init, start_col)?;
            Ok(())
        };
        if let Some(ty) = ty {
            self.out.token_space(":")?;
            return self.ty_tail(ty, Some(&self.tail_fn(init_else)));
        }
        init_else(self)?;
        Ok(())
    }

    fn local_else(
        &self,
        else_: &ast::Block,
        is_single_line_init: bool,
        start_col: HSize,
    ) -> FormatResult {
        let else_block_vertical = || {
            self.block_expr(true, else_)?;
            self.out.token(";")?;
            Ok(())
        };
        let same_line_else = || -> FormatResult {
            self.out.space_token_space("else")?;
            self.out.token("{")?;
            let else_block_horizontal = if is_single_line_init
                && let Some(expr_only_else) = self.try_into_optional_block(else_)
            {
                Some(move || {
                    self.with_single_line(|| {
                        self.with_width_limit_from_start(
                            start_col,
                            RUSTFMT_CONFIG_DEFAULTS.single_line_let_else_max_width,
                            || {
                                self.optional_block_horizontal_after_open_brace(expr_only_else)?;
                                self.out.token(";")?;
                                Ok(())
                            },
                        )
                    })
                })
            } else {
                None
            };
            self.backtrack()
                .next_opt(else_block_horizontal)
                .next(else_block_vertical)
                .result()?;
            Ok(())
        };
        let next_line_else = || -> FormatResult {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.out.token_space("else")?;
            self.out.token("{")?;
            else_block_vertical()?;
            Ok(())
        };
        self.backtrack()
            .next_if(is_single_line_init || self.out.last_line_is_closers(), || {
                self.out.with_recover_width(same_line_else)
            })
            .next(next_line_else)
            .result()?;
        Ok(())
    }
}
