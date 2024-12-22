use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::config::{ListConfig, ParamListConfig};
use crate::ast_formatter::list::{Braces, list};
use crate::constraints::INDENT_WIDTH;
use crate::error::FormatResult;
use crate::error::WidthLimitExceededError;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use crate::config::Config;

impl AstFormatter {
    pub fn dot_chain(&self, expr: &ast::Expr, tail: Tail<'_>, is_overflow: bool) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let (root, mut dot_chain) = dot_chain.split_first().unwrap();
        // todo usage is currently commented out
        if is_overflow {
            // single-line the whole thing
            self.with_single_line(|| {
                self.expr(root)?;
                dot_chain
                    .iter()
                    .try_for_each(|item| self.dot_chain_item(item, false))?;
                Ok(())
            })?;
            self.tail(tail)?;
            return Ok(());
        }
        let len_before_root = self.out.len();
        let line_before_root = self.out.line();
        self.expr(root)?;
        // write items while they start within the indent margin
        let same_line = loop {
            if self.out.line() != line_before_root {
                break false;
            }
            if self.out.len() > len_before_root + INDENT_WIDTH {
                break true;
            }
            let next;
            (next, dot_chain) = dot_chain.split_first().unwrap();
            self.dot_chain_item(next, true)?;
            if dot_chain.is_empty() {
                return self.tail(tail);
            }
        };
        if same_line {
            let start_len = self.out.len() - len_before_root;
            self.dot_chain_single_line_root(dot_chain, start_len, tail)
        } else {
            // each item on a separate line, no indent
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item, false)?;
            }
            self.tail(tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line_root(
        &self,
        dot_chain: &[&ast::Expr],
        start_len: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        self.fallback(|| self.dot_chain_single_line(dot_chain, start_len, tail))
            .next(|| self.dot_chain_hanging_indent(dot_chain, tail))
            .result()
    }

    fn dot_chain_single_line(
        &self,
        dot_chain: &[&ast::Expr],
        start_len: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        let width_limit = if self.config().rustfmt_quirks && dot_chain.len() == 1 {
            None
        } else {
            match RUSTFMT_CONFIG_DEFAULTS.chain_width.checked_sub(start_len) {
                None => return Err(WidthLimitExceededError.into()),
                Some(width) => Some(width),
            }
        };
        self.with_width_limit_first_line_opt(width_limit, || {
            let (last, until_last) = dot_chain.split_last().unwrap();
            self.with_single_line(|| {
                self.with_no_overflow(|| {
                    for item in until_last {
                        self.dot_chain_item(item, false)?;
                    }
                    Ok(())
                })?;
                let mut fallback =
                    self.fallback(|| self.with_no_overflow(|| self.dot_chain_item(last, false)));
                if fallback.is_done() {
                    return fallback.result();
                }
                fallback = fallback.next(|| {
                    self.indented(|| {
                        self.with_not_single_line(|| self.out.newline_indent())?;
                        self.with_no_overflow(|| self.dot_chain_item(last, false))?;
                        Ok(())
                    })
                });
                if fallback.peek_result().is_ok() {
                    // if wrapping makes the last item fit on one line,
                    // abort the single-line approach
                    return Err(WidthLimitExceededError.into());
                }
                // try with overflow
                fallback.next(|| self.dot_chain_item(last, false)).result()
            })
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_hanging_indent(&self, dot_chain: &[&ast::Expr], tail: Tail<'_>) -> FormatResult {
        self.indented(|| {
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item, false)?;
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_item(&self, expr: &ast::Expr, is_first_line: bool) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg, true)?;
                list(Braces::PARENS, &method_call.args, |arg| self.expr(arg))
                    .config(&MethodCallParamsListConfig { is_first_line })
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

pub struct MethodCallParamsListConfig {
    is_first_line: bool,
}

impl ListConfig for MethodCallParamsListConfig {
    fn single_line_max_contents_width(&self) -> Option<usize> {
        if self.is_first_line {
            None
        } else {
            Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
        }
    }

    fn overflow_max_first_line_contents_width(&self, config: &Config) -> Option<usize> {
        if config.rustfmt_quirks {
            Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width - 2)
        } else {
            Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
        }
    }
}

/*
impl X {
    fn test() {
        self.out.token_expect("if")?;
        self.out.space()?;
        self.fallback(|| self.with_single_line(|| self.expr_tail(scrutinee, Tail::OPEN_BLOCK)))
            .next(|| {
                self.expr(scrutinee)?;
                self.out.newline_indent()?;
                self.out.token_expect("{")?;
                Ok(())
            })
            .result()?;
        self.fallback(|| self.with_single_line(|| self.expr_tail(scrutinee, Tail::OPEN_BLOCK)));
        self.x()
            .fallback(|| self.with_single_line(|| self.expr_tail(scrutinee, Tail::OPEN_BLOCK)))
            .x();
    }
}
*/