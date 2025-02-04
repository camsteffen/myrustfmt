use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_wrappable, postfix_expr_receiver};
use crate::error::{ConstraintError, FormatError, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use std::ops::ControlFlow;

struct PostfixItem<'a> {
    /// The first item in the chain has the root expression, which is not a postfix expression.
    /// Subsequent items have a dot-notation item here, like `.field` or `.method()`.
    root_or_dot_item: &'a ast::Expr,
    /// any `?` or `[index]` occurring after `root_or_dot_item`
    tail: Vec<&'a ast::Expr>,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let start_pos = self.out.last_line_len();
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        self.with_width_limit_from_start_first_line(start_pos, width_limit, || {
            let chain = build_postfix_chain(expr);
            self.postfix_chain_given_width_limit(&chain, tail)
        })
    }

    fn postfix_chain_given_width_limit(
        &self,
        chain: &[PostfixItem<'_>],
        tail: &Tail,
    ) -> FormatResult {
        // touchy margins - everything must be a single line (with overflow)
        if self.out.constraints().touchy_margin.get() {
            self.postfix_chain_single_line_with_overflow(&chain, tail, false)?;
            return Ok(());
        }

        let first_line = self.out.line();
        let mut chain_rest = chain;

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            let Some((next, chain_rest_next)) = chain_rest.split_first() else {
                return self.tail(tail);
            };
            self.postfix_item(next)?;
            if chain_rest_next.is_empty() {
                return self.tail(tail);
            }
            chain_rest = chain_rest_next;
            if self.out.line() != first_line {
                break true;
            }
            if self.out.last_line_len() > indent_margin {
                break false;
            }
        };

        if multi_line_root {
            // each item on a separate line, no indent
            self.postfix_chain_separate_lines(chain_rest, tail)
        } else {
            self.backtrack()
                .next(|| self.postfix_chain_single_line_with_overflow(chain_rest, tail, true))
                .otherwise(|| self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail)))
        }
    }

    fn postfix_chain_single_line_with_overflow(
        &self,
        chain: &[PostfixItem<'_>],
        tail: &Tail,
        // todo factor out bool?
        has_separate_lines_fallback: bool,
    ) -> FormatResult {
        let last = chain.last().unwrap();
        if !matches!(last.root_or_dot_item.kind, ast::ExprKind::MethodCall(_)) {
            // this chain is not overflowable, so simply format it on one line
            self.with_single_line(|| self.postfix_items(chain))?;
            self.tail(tail)?;
            return Ok(());
        }
        let (overflowable, until_overflow) = chain.split_last().unwrap();

        self.with_single_line(|| self.postfix_items(until_overflow))?;

        if !has_separate_lines_fallback {
            // todo consider comparing line count with adding a block for touchy_margins
            self.postfix_overflowable(overflowable)?;
            self.tail(tail)?;
            return Ok(());
        }

        let wrappable_count = until_overflow.len() as u32;
        self.postfix_chain_overflow_last_unless_separate_lines_preferred(
            overflowable,
            tail,
            wrappable_count,
        )?;
        Ok(())
    }

    fn postfix_chain_overflow_last_unless_separate_lines_preferred(
        &self,
        overflowable: &PostfixItem<'_>,
        tail: &Tail,
        wrappable_count: u32,
    ) -> FormatResult {
        let first_line = self.out.line();
        let result = self.backtrack().next_control_flow_lookahead(|| {
            let result = (|| -> FormatResult {
                self.postfix_overflowable(overflowable)?;
                self.tail(tail)?;
                Ok(())
            })();
            match result {
                Err(e) => return ControlFlow::Break(Err(e)),
                Ok(()) => {}
            }
            if self.out.line() - first_line == 1 {
                // it all fits on one line
                return ControlFlow::Break(Ok(()));
            }
            ControlFlow::Continue(self.out.line() - first_line + 1)
        });
        let (backtrack, lookahead, overflow_height) = match result {
            ControlFlow::Break(result) => return result,
            ControlFlow::Continue(c) => c,
        };

        // try writing the overflowable on the next line to measure its height in separate lines format
        let result = backtrack.next_control_flow(|| {
            // todo share logic with match arm
            // todo should we check if the wrap allows a *longer* first line, like match arm?
            let result = self.indented(|| {
                // N.B. this newline evades the first line width limit
                self.out.line();
                self.out.newline_within_indent()?;
                self.postfix_item(overflowable)?;
                self.tail(tail)?;
                Ok(self.out.line() - first_line + 1)
            });
            match result {
                Err(FormatError::Constraint(_)) => todo!(), // is this possible?
                Err(e) => ControlFlow::Break(Err(e)),
                Ok(overflowable_separate_line_height) => {
                    // Each wrappable leading up to the overflowable will occupy one line each since we were
                    // previously able to format them all on one line.
                    let separate_lines_height = wrappable_count + overflowable_separate_line_height;
                    if separate_lines_height <= overflow_height {
                        // fallback to separate lines strategy
                        return ControlFlow::Break(Err(ConstraintError::Logical.into()));
                    }
                    // restore overflow strategy
                    ControlFlow::Continue(())
                }
            }
            // todo perhaps there is a concept of "fallback cost" that can be passed down
            //   - if you err out, there will be a cost of increased indent and added lines
        });

        match result {
            ControlFlow::Break(result) => result,
            ControlFlow::Continue((backtrack, ())) => backtrack.otherwise_lookahead(&lookahead, ()),
        }
    }

    fn postfix_overflowable(&self, overflowable: &PostfixItem<'_>) -> FormatResult {
        self.postfix_item_root(overflowable.root_or_dot_item)?;
        self.with_single_line(|| self.postfix_tail(&overflowable.tail))?;
        Ok(())
    }

    fn postfix_chain_separate_lines(&self, chain: &[PostfixItem<'_>], tail: &Tail) -> FormatResult {
        for item in chain {
            self.out.newline_within_indent()?;
            self.postfix_item(item)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn postfix_item(&self, item: &PostfixItem<'_>) -> FormatResult {
        self.postfix_item_root(item.root_or_dot_item)?;
        self.postfix_tail(&item.tail)?;
        Ok(())
    }

    fn postfix_item_root(&self, item: &ast::Expr) -> FormatResult {
        match item.kind {
            ast::ExprKind::Await(..) => {
                self.out.token(".")?;
                self.out.token("await")?
            }
            ast::ExprKind::Field(_, ident) => {
                self.out.token(".")?;
                self.ident(ident)?
            }
            ast::ExprKind::MethodCall(ref method_call) => {
                self.out.token(".")?;
                self.path_segment(&method_call.seg, true, &Tail::token("("))?;
                // todo this is consistent with rustfmt, but would it be better to force args to be
                //   on the same line, just allowing overflow of the last arg?
                self.call_args_after_open_paren(&method_call.args, Tail::none())
                    .format(self)?;
            }
            // root expression
            _ => self.expr(item)?,
        }
        Ok(())
    }

    fn postfix_items(&self, items: &[PostfixItem<'_>]) -> FormatResult {
        items.iter().try_for_each(|item| self.postfix_item(item))
    }

    fn postfix_tail(&self, tail: &[&ast::Expr]) -> FormatResult {
        for expr in tail {
            match expr.kind {
                ast::ExprKind::Index(_, ref index, _) => {
                    self.out.token("[")?;
                    self.backtrack()
                        .next(|| self.with_single_line(|| self.expr_tail(index, &Tail::token("]"))))
                        .otherwise(|| self.embraced_after_opening("]", || self.expr(index)))?;
                }
                ast::ExprKind::Try(..) => self.out.token("?")?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

fn build_postfix_chain(expr: &ast::Expr) -> Vec<PostfixItem<'_>> {
    let mut current = expr;
    let mut items = Vec::new();
    let mut tail = Vec::new();
    let root = loop {
        if postfix_expr_is_wrappable(current) {
            items.push(PostfixItem {
                root_or_dot_item: current,
                tail: tail.drain(..).rev().collect(),
            })
        } else {
            tail.push(current);
        }
        let receiver = postfix_expr_receiver(current);
        if !is_postfix_expr(receiver) {
            break receiver;
        }
        current = receiver;
    };
    tail.reverse();
    items.push(PostfixItem {
        root_or_dot_item: root,
        tail,
    });
    items.reverse();
    items
}
