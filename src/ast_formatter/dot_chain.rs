use tracing::info;
use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{list, param_list_config};
use crate::constraint_writer::TooWideError;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::source_formatter::{FormatError, FormatResult};

impl AstFormatter {
    pub fn dot_chain(
        &self,
        expr: &ast::Expr,
        tail: Tail<'_>,
        is_overflow: bool,
    ) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let [root, dot_chain @ ..] = &dot_chain[..] else {
            unreachable!();
        };
        if is_overflow {
            // single-line the whole thing
            self.with_single_line(|| {
                self.expr(root, Tail::NONE)?;
                dot_chain
                    .iter()
                    .try_for_each(|item| self.dot_chain_item(item))?;
                Ok(())
            })?;
            self.tail(tail)?;
            return Ok(());
        }
        let (len, height) = self.with_dimensions(|| self.expr(root, Tail::NONE))?;
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
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|| {
                    let format = || {
                        let [until_last @ .., last] = dot_chain else {
                            unreachable!()
                        };
                        self.with_single_line(|| {
                            self.with_no_overflow(|| {
                                for item in until_last {
                                    self.dot_chain_item(item)?;
                                }
                                Ok(())
                            })?;
                            let before_last = self.out.snapshot();
                            let no_overflow_result = self.with_no_overflow(|| {
                                self.dot_chain_item(last)
                            });
                            if matches!(no_overflow_result, Ok(())) {
                                info!("no overflow ok");
                                return Ok(())
                            }
                            self.out.restore(&before_last);
                            let wrap_result = self.indented(|| {
                                self.with_not_single_line(|| {
                                    self.out.newline_indent()
                                })?;
                                self.with_no_overflow(|| {
                                    self.dot_chain_item(last)
                                })?;
                                Ok(())
                            });
                            if wrap_result.is_ok() {
                                // if wrapping makes the last item fit on one line,
                                // abort the single-line approach
                                return Err(self.out.lift_constraint_err(TooWideError));
                            }
                            self.out.restore(&before_last);
                            // try with overflow
                            self.dot_chain_item(last)?;
                            Ok(())
                        })
                    };
                    if dot_chain.len() > 1 {
                        let Some(width) = RUSTFMT_CONFIG_DEFAULTS.chain_width.checked_sub(root_len)
                        else {
                            return Err(self.out.lift_constraint_err(TooWideError));
                        };
                        self.with_width_limit_first_line(width, format)
                    } else {
                        format()
                    }
                });
                // wrap and indent each item
                chain.next(|| {
                    self.indented(|| {
                        for item in dot_chain {
                            self.out.newline_indent()?;
                            self.dot_chain_item(item)?;
                        }
                        Ok(())
                    })
                })
            },
            move || self.tail(tail),
        )?;
        Ok(())
    }

    fn dot_chain_item(&self, expr: &ast::Expr) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg)?;
                list(
                    &method_call.args,
                    |arg| self.expr(arg, Tail::NONE),
                    param_list_config(Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)),
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
