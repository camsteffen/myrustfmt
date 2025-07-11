use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::width_thresholds::WIDTH_THRESHOLDS;
use crate::error::FormatResult;
use crate::num::{HSize, VSize};
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn local(&self, local: &ast::Local) -> FormatResult {
        self.with_attrs(&local.attrs, local.span.into(), || {
            self.local_after_attrs(local)
        })
    }

    fn local_after_attrs(&self, local: &ast::Local) -> FormatResult {
        let (first_line, start_col) = self.out.line_col();
        self.out.token_space("let")?;
        self.pat_tail(
            &local.pat,
            Some(&self.tail_fn(|af| af.local_after_pat(local, first_line, start_col))),
        )?;
        Ok(())
    }

    fn local_after_pat(
        &self,
        local: &ast::Local,
        first_line: VSize,
        start_col: HSize,
    ) -> FormatResult {
        let after_ty = |af: &Self| af.local_after_ty(local, first_line, start_col);
        if let Some(ty) = &local.ty {
            self.out.token_space(":")?;
            self.ty_tail(ty, Some(&self.tail_fn(after_ty)))?;
        } else {
            after_ty(self)?;
        }
        Ok(())
    }

    fn local_after_ty(
        &self,
        local: &ast::Local,
        first_line: VSize,
        start_col: HSize,
    ) -> FormatResult {
        let Some((init, else_)) = local.kind.init_else_opt() else {
            return self.out.token(";");
        };
        self.assign_expr(init, else_.is_none().then(|| self.tail_token(";")).as_ref())?;
        let Some(else_) = else_ else { return Ok(()) };
        let is_single_line_init = self.out.line() == first_line;
        self.local_else(else_, is_single_line_init, start_col)?;
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
                && let Some(expr_only_else) = self.try_into_expr_only_block(else_)
            {
                Some(move |_: &_| {
                    let _guard = self.single_line_guard();
                    let _guard = self.width_limit_end_guard(
                        start_col + WIDTH_THRESHOLDS.single_line_let_else_max_width,
                    )?;
                    self.optional_block_horizontal_after_open_brace(expr_only_else)?;
                    self.out.token(";")?;
                    Ok(())
                })
            } else {
                None
            };
            self.backtrack()
                .next_opt(else_block_horizontal)
                .next(|_| else_block_vertical())
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
            .next_if(
                is_single_line_init || self.out.last_line_is_closers(),
                |_| {
                    let _guard = self.recover_width_guard();
                    same_line_else()?;
                    Ok(())
                },
            )
            .next(|_| next_line_else())
            .result()?;
        Ok(())
    }
}
