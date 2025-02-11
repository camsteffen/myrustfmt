use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::builder::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::error::{ConstraintError, FormatError, FormatResult, FormatResultExt, return_if_break};
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
        let first_line = self.out.line();
        let mut chain_rest = chain;

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            let Some((next, chain_rest_next)) = chain_rest.split_first() else {
                return self.tail(tail);
            };
            chain_rest = chain_rest_next;
            if chain_rest.is_empty() {
                self.postfix_item(next)?;
                return self.tail(tail);
            }
            let next_is_single_line = self.constraints().requires_indent_middle();
            self.with_single_line_opt(next_is_single_line, || self.postfix_item(next))?;
            if self.out.line() != first_line {
                // should be prevented by single-line constraint above
                assert_eq!(self.constraints().requires_indent_middle(), false);
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
                .next(|| self.postfix_chain_single_line_with_overflow(chain_rest, tail))
                .otherwise(|| {
                    // see docs for MultiLineConstraint
                    self.with_single_line_opt(
                        self.constraints().requires_single_line_chains(),
                        || self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail)),
                    )
                })
        }
    }

    fn postfix_chain_single_line_with_overflow(
        &self,
        chain: &[PostfixItem<'_>],
        tail: &Tail,
    ) -> FormatResult {
        let last = chain.last().unwrap();
        if !matches!(last.root_or_dot_item.kind, ast::ExprKind::MethodCall(_)) {
            // this chain is not overflowable, so simply format it on one line
            self.with_single_line(|| self.postfix_items(chain))?;
            self.tail(tail)?;
            return Ok(());
        }
        let (overflowable, before_overflow) = chain.split_last().unwrap();

        self.with_single_line(|| self.postfix_items(before_overflow))?;

        let before_overflow_count = before_overflow.len() as u32;
        self.postfix_chain_overflow_last_unless_separate_lines_preferred(
            overflowable,
            tail,
            before_overflow_count,
        )?;
        Ok(())
    }

    fn postfix_chain_overflow_last_unless_separate_lines_preferred(
        &self,
        overflowable: &PostfixItem<'_>,
        tail: &Tail,
        before_overflow_count: u32,
    ) -> FormatResult {
        let first_line = self.out.line();
        let checkpoint = self.open_checkpoint();

        let result = self.maybe_lookahead(checkpoint, || {
            self.postfix_overflowable(overflowable).break_err()?;
            self.tail(tail).break_err()?;
            if self.out.line() == first_line {
                // it all fits on one line
                return ControlFlow::Break(Ok(()));
            }
            // todo can we prove that the overflow is so long that a separate line won't be shorter?
            ControlFlow::Continue(self.out.line() - first_line + 1)
        });
        let (checkpoint, overflow_lookahead, overflow_height) = return_if_break!(result);

        // try writing the overflowable on the next line to measure its height in separate lines format
        // todo share logic with match arm
        // todo should we check if the wrap allows a *longer* first line, like match arm?
        // todo account for having less width if the fallback will add a block
        let result = self.indented(|| {
            // N.B. this newline evades the first line width limit
            self.out.newline_within_indent()?;
            self.postfix_item(overflowable)?;
            self.tail(tail)?;
            Ok(self.out.line() - first_line + 1)
        });

        let result = match result {
            Err(FormatError::Constraint(_)) => todo!(), // is this possible?
            Err(e) => Err(e),
            Ok(separate_lines_height) => {
                if separate_lines_height <= overflow_height {
                    // fallback to separate lines strategy
                    Err(ConstraintError::Logical.into())
                } else {
                    self.restore_checkpoint(&checkpoint);
                    self.restore_lookahead(&overflow_lookahead);
                    Ok(())
                }
            }
        };

        self.close_checkpoint(checkpoint);
        result
        // todo perhaps there is a concept of "fallback cost" that can be passed down
        //   - if you err out, there will be a cost of increased indent and added lines
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
        if postfix_expr_is_dot(current) {
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
