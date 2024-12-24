use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::config::ListConfig;
use crate::ast_formatter::list::{Braces, list};
use crate::config::Config;
use crate::constraints::INDENT_WIDTH;
use crate::error::WidthLimitExceededError;
use crate::error::{FormatResult, FormatResultExt};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use tracing::info;

impl AstFormatter {
    pub fn dot_chain(&self, expr: &ast::Expr, tail: Tail<'_>) -> FormatResult {
        info!(
            "single line: {:?}, max_width: {:?}",
            self.out.constraints().single_line.get(),
            self.out.constraints().max_width.get()
        );
        // info!("{}", self.out.constraints().single_line_backtrace.borrow().as_ref().unwrap());
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let (root, mut dot_chain) = dot_chain.split_first().unwrap();
        let first_line = self.out.line();
        let start_pos = self.out.last_line_len();
        self.expr(root)?;
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        while self.out.line() == first_line && self.out.last_line_len() <= indent_margin {
            info!("in the margin");
            let next;
            (next, dot_chain) = dot_chain.split_first().unwrap();
            self.dot_chain_item(next, true, true, true)?;
            if dot_chain.is_empty() {
                return self.tail(tail);
            }
        }
        if self.out.line() == first_line {
            self.fallback(|| self.dot_chain_single_line(dot_chain, start_pos, tail))
                .next(|| self.dot_chain_separate_lines_indented(dot_chain, tail))
                .result()
        } else {
            // each item on a separate line, no indent
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item, false, true, true)?;
            }
            self.tail(tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line(
        &self,
        dot_chain: &[&ast::Expr],
        start_pos: usize,
        tail: Tail<'_>,
    ) -> FormatResult {
        let width_limit = if self.config().rustfmt_quirks && dot_chain.len() == 1 {
            None
        } else {
            Some(RUSTFMT_CONFIG_DEFAULTS.chain_width)
        };
        let (last, until_last) = dot_chain.split_last().unwrap();
        self.with_width_limit_from_start_first_line_opt(start_pos, width_limit, || {
            self.with_single_line(|| {
                for item in until_last {
                    self.dot_chain_item(item, false, false, false)?;
                }
                Ok(())
            })
        })?;
        let snapshot = self.out.snapshot();
        // no multiline overflow
        let result = self
            .with_width_limit_from_start_first_line_opt(start_pos, width_limit, || {
                self.with_single_line(|| self.dot_chain_item(last, false, false, false))
            })
            .and_then(|()| self.tail(tail));
        if result.is_ok_or_parse_error() {
            // all fits on one line (or a critical error)
            info!("first try lol");
            return result;
        }
        self.out.restore(&snapshot);

        // experimentally check if wrapping the last item makes it fit on one line
        // the width limit would not apply
        // todo wouldn't the first line width limit be removed anyways from the newline?
        info!("trying wrap");
        let result = self.indented(|| {
            self.out.newline_indent()?;
            info!("initial wrapped length: {}", self.out.last_line_len());
            info!("max width: {:?}", self.out.constraints().max_width.get());
            self.with_single_line(|| {
                self.dot_chain_item(last, false, false, false)?;
                self.tail(tail)?;
                FormatResult::Ok(())
            })?;
            Ok(())
        });
        info!("wrap result: {result:?}");
        if result.is_ok() {
            // ...if so, go to the separate lines approach
            return Err(WidthLimitExceededError.into());
        }
        self.out.restore(&snapshot);
        self.with_width_limit_from_start_first_line_opt(start_pos, width_limit, || {
            self.with_single_line(|| {
                // try with overflow
                info!("trying overflow");
                self.dot_chain_item(last, false, true, true)
            })
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_separate_lines_indented(
        &self,
        dot_chain: &[&ast::Expr],
        tail: Tail<'_>,
    ) -> FormatResult {
        info!("separate lines now");
        self.indented(|| {
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item, false, true, true)?;
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_item(
        &self,
        expr: &ast::Expr,
        is_first_line: bool,
        overflow: bool,
        allow_multiline_overflow: bool,
    ) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            // todo share code with ExprKind::Call?
            ast::ExprKind::MethodCall(ref method_call) => {
                self.with_no_multiline_overflow_optional(!allow_multiline_overflow, || {
                    self.path_segment(&method_call.seg, true)?;
                    let list_config = MethodCallParamsListConfig { is_first_line };
                    let list = list(Braces::PARENS, &method_call.args, |arg| self.expr(arg))
                        .config(&list_config);
                    if overflow || true {
                        list.overflow().format(self)?;
                    } else {
                        list.format(self)?;
                    }
                    Ok(())
                })
            }
            _ => unreachable!(),
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
