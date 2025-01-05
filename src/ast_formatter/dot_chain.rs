use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::ListConfig;
use crate::ast_formatter::list::{Braces, list};
use crate::ast_formatter::util::tail::Tail;
use crate::config::Config;
use crate::constraints::INDENT_WIDTH;
use crate::error::WidthLimitExceededError;
use crate::error::{FormatResult, FormatResultExt};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use tracing::info;

struct DotChainItem<'a> {
    expr: &'a ast::Expr,
    /// Number of trailing `?` operators
    try_ops: u32,
}

impl AstFormatter {
    pub fn dot_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let items = build_dot_chain(expr);
        let (root, dot_chain) = items.split_first().unwrap();
        let first_line = self.out.line();
        let start_pos = self.out.last_line_len();
        self.expr(root.expr)?;
        self.trys(root.try_ops)?;
        if dot_chain.is_empty() {
            self.tail(tail)?;
            return Ok(());
        }
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let mut dot_chain_remaining = dot_chain;
        while self.out.line() == first_line && self.out.last_line_len() <= indent_margin {
            let next;
            (next, dot_chain_remaining) = dot_chain_remaining.split_first().unwrap();
            self.dot_chain_item(next)?;
            if dot_chain_remaining.is_empty() {
                return self.tail(tail);
            }
        }
        if self.out.line() == first_line {
            self.fallback(|| self.dot_chain_single_line(dot_chain_remaining, start_pos, tail))
                .next(|| self.dot_chain_separate_lines_indented(dot_chain_remaining, tail))
                .result()
        } else {
            // each item on a separate line, no indent
            for item in dot_chain_remaining {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            self.tail(tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line(
        &self,
        dot_chain: &[DotChainItem<'_>],
        start_pos: usize,
        tail: &Tail,
    ) -> FormatResult {
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        let (last, until_last) = dot_chain.split_last().unwrap();
        self.with_width_limit_from_start(start_pos, width_limit, || {
            self.with_single_line(|| {
                for item in until_last {
                    self.dot_chain_item(item)?;
                }
                Ok(())
            })
        })?;
        let snapshot = self.out.snapshot();
        // no multiline overflow
        let result = self
            .with_width_limit_from_start(start_pos, width_limit, || {
                self.with_single_line(|| self.dot_chain_item(last))
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
                self.dot_chain_item(last)?;
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
        self.with_width_limit_from_start_first_line(start_pos, width_limit, || {
            // try with overflow
            info!("trying overflow");
            self.dot_chain_item(last)
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_separate_lines_indented(
        &self,
        dot_chain: &[DotChainItem<'_>],
        tail: &Tail,
    ) -> FormatResult {
        info!("separate lines now");
        self.indented(|| {
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn dot_chain_item(&self, item: &DotChainItem<'_>) -> FormatResult {
        self.out.token(".")?;
        match item.expr.kind {
            ast::ExprKind::Await(..) => self.out.token("await")?,
            ast::ExprKind::Field(_, ident) => self.ident(ident)?,
            // todo share code with ExprKind::Call?
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg, true)?;
                list(Braces::PARENS, &method_call.args, |arg| self.expr(arg))
                    .config(&MethodCallParamsListConfig)
                    .overflow()
                    .format(self)?;
            }
            _ => unreachable!(),
        }
        self.trys(item.try_ops)?;
        Ok(())
    }

    fn trys(&self, try_ops: u32) -> FormatResult {
        (0..try_ops).try_for_each(|_| self.out.token("?"))
    }
}

fn build_dot_chain(expr: &ast::Expr) -> Vec<DotChainItem<'_>> {
    let mut items = Vec::new();
    do_build_dot_chain(&mut items, expr, 0);
    items
}

fn do_build_dot_chain<'a>(chain: &mut Vec<DotChainItem<'a>>, expr: &'a ast::Expr, try_ops: u32) {
    let mut dot = |inner| {
        do_build_dot_chain(chain, inner, 0);
        chain.push(DotChainItem { expr, try_ops });
    };
    match &expr.kind {
        ast::ExprKind::Await(target, _) | ast::ExprKind::Field(target, _) => dot(target),
        ast::ExprKind::MethodCall(method_call) => dot(&method_call.receiver),
        ast::ExprKind::Try(inner) => do_build_dot_chain(chain, inner, try_ops + 1),
        _ => chain.push(DotChainItem { expr, try_ops }),
    };
}

pub struct MethodCallParamsListConfig;

impl ListConfig for MethodCallParamsListConfig {
    fn overflow_max_first_line_contents_width(&self, _config: &Config) -> Option<u32> {
        Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
    }

    fn single_line_max_contents_width(&self) -> Option<u32> {
        Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
    }
}
