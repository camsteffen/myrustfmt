use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::ListConfig;
use crate::ast_formatter::list::{Braces, list};
use crate::ast_formatter::util::tail::Tail;
use crate::config::Config;
use crate::constraints::INDENT_WIDTH;
use crate::error::FormatResult;
use crate::error::{ConstraintError, FormatError, WidthLimitExceededError};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_span::symbol::Ident;
use tracing::info;

#[derive(Debug)]
enum PostfixChainItem<'a> {
    Await,
    Field(Ident),
    // todo don't overflow index when single line chain
    Index(&'a ast::Expr),
    MethodCall(&'a ast::MethodCall),
    Try,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let (root, chain) = build_postfix_chain(expr);
        let first_line = self.out.line();
        let start_pos = self.out.last_line_len();
        self.expr(root)?;
        let chain = self.chain_unbreakables(&chain)?;
        if chain.is_empty() {
            self.tail(tail)?;
            return Ok(());
        }
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let mut chain_remaining = chain;
        while self.out.line() == first_line && self.out.last_line_len() <= indent_margin {
            chain_remaining = self.chain_item_and_unbreakables(chain_remaining)?;
            if chain_remaining.is_empty() {
                return self.tail(tail);
            }
        }
        if self.out.line() != first_line {
            // each item on a separate line, no indent
            return self.postfix_chain_separate_lines(chain_remaining, tail);
        }
        self.fallback(|| self.postfix_chain_single_line(chain_remaining, start_pos, tail))
            .next(|| self.indented(|| self.postfix_chain_separate_lines(chain_remaining, tail)))
            .result()
    }

    fn postfix_chain_single_line(
        &self,
        chain: &[PostfixChainItem<'_>],
        start_pos: usize,
        tail: &Tail,
    ) -> FormatResult {
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        let items_single_line = |chain| {
            self.with_width_limit_from_start(start_pos, width_limit, || {
                self.with_single_line(|| self.postfix_chain_items(chain))
            })
        };
        let Some((until_overflow, overflowable)) = split_overflowable(chain) else {
            items_single_line(chain)?;
            self.tail(tail)?;
            return Ok(());
        };
        items_single_line(until_overflow)?;
        let snapshot = self.out.snapshot();
        let result = items_single_line(overflowable).and_then(|()| self.tail(tail));
        match result {
            Ok(()) | Err(FormatError::Parse(_)) => return result,
            Err(FormatError::Constraint(_)) => {}
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
                self.postfix_chain_items(overflowable)?;
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
            let (first, rest) = overflowable.split_first().unwrap();
            self.postfix_chain_item(first)?;
            self.with_single_line(|| self.postfix_chain_items(rest))?;
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn postfix_chain_separate_lines(
        &self,
        mut chain: &[PostfixChainItem<'_>],
        tail: &Tail,
    ) -> FormatResult {
        while !chain.is_empty() {
            self.out.newline_indent()?;
            chain = self.chain_item_and_unbreakables(chain)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn chain_item_and_unbreakables<'a, 'b>(
        &self,
        chain: &'b [PostfixChainItem<'a>],
    ) -> FormatResult<&'b [PostfixChainItem<'a>]> {
        let (first, mut rest) = chain.split_first().unwrap();
        self.postfix_chain_item(first)?;
        rest = self.chain_unbreakables(rest)?;
        Ok(rest)
    }

    fn chain_unbreakables<'a, 'b>(
        &self,
        mut chain: &'b [PostfixChainItem<'a>],
    ) -> FormatResult<&'b [PostfixChainItem<'a>]> {
        loop {
            match chain {
                [next, rest @ ..] if is_unbreakable(next) => {
                    chain = rest;
                    self.postfix_chain_item(next)?;
                }
                _ => break,
            }
        }
        Ok(chain)
    }

    fn postfix_chain_item(&self, item: &PostfixChainItem<'_>) -> FormatResult {
        match *item {
            PostfixChainItem::Await => {
                self.out.token(".")?;
                self.out.token("await")?
            }
            PostfixChainItem::Field(ident) => {
                self.out.token(".")?;
                self.ident(ident)?
            }
            // todo share code with ExprKind::Call?
            PostfixChainItem::MethodCall(method_call) => {
                self.out.token(".")?;
                self.path_segment(&method_call.seg, true)?;
                list(Braces::PARENS, &method_call.args, |arg| self.expr(arg))
                    .config(&MethodCallParamsListConfig)
                    .overflow()
                    .format(self)?;
            }
            PostfixChainItem::Index(index) => {
                self.out.token("[")?;
                self.fallback(|| {
                    self.with_single_line(|| self.expr_tail(index, &Tail::token("]")))
                })
                .next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.expr(index)?;
                        Ok(())
                    })?;
                    self.out.newline_indent()?;
                    self.out.token("]")?;
                    Ok(())
                })
                .result()?;
            }
            PostfixChainItem::Try => {
                self.out.token("?")?;
            }
        }
        Ok(())
    }

    fn postfix_chain_items(&self, items: &[PostfixChainItem<'_>]) -> FormatResult {
        items
            .iter()
            .try_for_each(|item| self.postfix_chain_item(item))
    }
}

fn build_postfix_chain(mut expr: &ast::Expr) -> (&ast::Expr, Vec<PostfixChainItem<'_>>) {
    let mut items = Vec::new();
    loop {
        match expr.kind {
            ast::ExprKind::Await(ref target, _) => {
                items.push(PostfixChainItem::Await);
                expr = target;
            }
            ast::ExprKind::Field(ref target, ident) => {
                items.push(PostfixChainItem::Field(ident));
                expr = target;
            }
            ast::ExprKind::Index(ref target, ref index, _) => {
                items.push(PostfixChainItem::Index(index));
                expr = target;
            }
            ast::ExprKind::MethodCall(ref method_call) => {
                items.push(PostfixChainItem::MethodCall(method_call));
                expr = &method_call.receiver;
            }
            ast::ExprKind::Try(ref target) => {
                items.push(PostfixChainItem::Try);
                expr = target;
            }
            _ => break,
        };
    }
    items.reverse();
    (expr, items)
}

fn split_overflowable<'a, 'b>(
    chain: &'b [PostfixChainItem<'a>],
) -> Option<(&'b [PostfixChainItem<'a>], &'b [PostfixChainItem<'a>])> {
    let mut count = 0;
    for item in chain.iter().rev() {
        if is_unbreakable(item) {
            count += 1;
        } else if let PostfixChainItem::MethodCall(_) = item {
            count += 1;
            return Some(chain.split_at(chain.len() - count));
        } else {
            break;
        }
    }
    None
}

fn is_unbreakable(item: &PostfixChainItem<'_>) -> bool {
    matches!(item, PostfixChainItem::Index(_) | PostfixChainItem::Try)
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
