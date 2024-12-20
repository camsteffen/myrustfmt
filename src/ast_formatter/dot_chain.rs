use crate::RUSTFMT_QUIRKS;
use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{list, param_list_config};
use crate::constraints::INDENT_WIDTH;
use crate::error::FormatResult;
use crate::error::WidthLimitExceededError;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use crate::ast_formatter::fallback_chain::ResultFallback;

impl AstFormatter {
    pub fn dot_chain(&self, expr: &ast::Expr, tail: Tail<'_>, is_overflow: bool) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let (root, dot_chain) = dot_chain.split_first().unwrap();
        if is_overflow {
            // single-line the whole thing
            self.with_single_line(|| {
                self.expr(root)?;
                dot_chain
                    .iter()
                    .try_for_each(|item| self.dot_chain_item(item))?;
                Ok(())
            })?;
            self.tail(tail)?;
            return Ok(());
        }
        let (len, height) = self.with_dimensions(|| self.expr(root))?;
        if height == 0 {
            self.dot_chain_single_line_root(dot_chain, len, tail)
        } else {
            // each item on a separate line, no indent
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            self.tail(tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line_root(
        &self,
        dot_chain: &[&ast::Expr],
        root_len: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        let snapshot = &self.out.snapshot();
        self.dot_chain_single_line(dot_chain, root_len, tail)
            .fallback(self, snapshot, || {
                self.dot_chain_separate_lines(dot_chain, root_len, tail)
            })
    }

    fn dot_chain_single_line(
        &self,
        dot_chain: &[&ast::Expr],
        root_len: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        let format = || {
            let (last, until_last) = dot_chain.split_last().unwrap();
            self.with_single_line(|| {
                self.with_no_overflow(|| {
                    for item in until_last {
                        self.dot_chain_item(item)?;
                    }
                    Ok(())
                })?;
                let before_last = self.out.snapshot();
                let no_overflow_result = self.with_no_overflow(|| self.dot_chain_item(last));
                if matches!(no_overflow_result, Ok(())) {
                    return Ok(());
                }
                self.out.restore(&before_last);
                let wrap_result = self.indented(|| {
                    self.with_not_single_line(|| self.out.newline_indent())?;
                    self.with_no_overflow(|| self.dot_chain_item(last))?;
                    Ok(())
                });
                if wrap_result.is_ok() {
                    // if wrapping makes the last item fit on one line,
                    // abort the single-line approach
                    return Err(self.out.format_error(WidthLimitExceededError));
                }
                self.out.restore(&before_last);
                // try with overflow
                self.dot_chain_item(last)?;
                Ok(())
            })
        };
        if RUSTFMT_QUIRKS && dot_chain.len() == 1 {
            format()?;
        } else {
            let Some(width) = RUSTFMT_CONFIG_DEFAULTS.chain_width.checked_sub(root_len) else {
                return Err(self.out.format_error(WidthLimitExceededError));
            };
            self.with_width_limit_first_line(width, format)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_separate_lines(
        &self,
        dot_chain: &[&ast::Expr],
        root_len: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        let rest = if root_len > INDENT_WIDTH {
            dot_chain
        } else {
            let (first, rest) = dot_chain.split_first().unwrap();
            self.dot_chain_item(first)?;
            rest
        };
        self.indented(|| {
            for item in rest {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_item(&self, expr: &ast::Expr) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg, true)?;
                list(
                    &method_call.args,
                    |arg| self.expr(arg),
                    // param_list_config(Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)),
                    param_list_config(None),
                )
                .overflow()
                .format(self)?;
                Ok(())
            }
            _ => unreachable!("invalid dot chain item ExprKind"),
        }
    }
}

fn build_dot_chain<'a>(chain: &mut Vec<&'a ast::Expr>, expr: &'a ast::Expr) {
    let inner = match &expr.kind {
        ast::ExprKind::Field(target, _ident) => Some(target),
        ast::ExprKind::MethodCall(method_call) => Some(&method_call.receiver),
        _ => None,
    };
    if let Some(inner) = inner {
        build_dot_chain(chain, inner);
    }
    chain.push(expr);
}
