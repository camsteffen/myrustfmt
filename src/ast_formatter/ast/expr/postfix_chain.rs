use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_formatter::tail::Tail;
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::constraints::MultiLineShape;
use crate::error::{ConstraintErrorKind, FormatResult, FormatResultExt};
use rustc_ast::ast;

// In rustfmt, this is called chain_width, and is 60 by default
const POSTFIX_CHAIN_MAX_WIDTH: u32 = 60;
/// Don't apply chain max width unless the chain item's distance from the start
/// of the chain is at least this much.
const POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH: u32 = 15;

struct PostfixItem<'a> {
    /// The first item in the chain has the root expression, which is not a postfix expression.
    /// Subsequent items have a dot-notation item here, like `.field` or `.method()`.
    root_or_dot_item: &'a ast::Expr,
    /// any `?` or `[index]` occurring after `root_or_dot_item`
    tail: Vec<&'a ast::Expr>,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let chain = build_postfix_chain(expr);
        let start_pos = self.out.last_line_len();
        let first_line = self.out.line();

        // items that start within the first indent-width on the first line
        let mut chain_iter = chain.iter();
        let indent_margin = self.out.indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            let next = chain_iter.next().unwrap();
            if chain_iter.as_slice().is_empty() {
                self.postfix_item(next)?;
                return self.tail(tail);
            }
            self.constraints()
                .with_single_line_unless(MultiLineShape::Unrestricted, || self.postfix_item(next))?;
            if self.out.line() != first_line {
                break true;
            }
            if self.out.last_line_len() > indent_margin {
                break false;
            }
        };
        let chain_rest = chain_iter.as_slice();

        if multi_line_root {
            // each item on a separate line, no indent
            self.postfix_chain_separate_lines(chain_rest, tail)
        } else {
            self.backtrack()
                .next(|| self.postfix_chain_single_line_with_overflow(chain_rest, start_pos, tail))
                .otherwise(|| {
                    self.constraints()
                        .with_single_line_unless(MultiLineShape::HangingIndent, || {
                            self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail))
                        })
                })
        }
    }

    fn postfix_chain_single_line_with_overflow(
        &self,
        chain: &[PostfixItem<'_>],
        start_pos: u32,
        tail: &Tail,
    ) -> FormatResult {
        let last = chain.last().unwrap();
        if !matches!(last.root_or_dot_item.kind, ast::ExprKind::MethodCall(_)) {
            // this chain is not overflowable, so simply format it on one line
            self.with_single_line(|| self.postfix_items(chain, start_pos))?;
            self.tail(tail)?;
            return Ok(());
        }
        let (overflowable, before_overflow) = chain.split_last().unwrap();

        self.with_single_line(|| self.postfix_items(before_overflow, start_pos))?;

        self.postfix_chain_overflow_last_unless_separate_lines_preferred(
            overflowable,
            start_pos,
            tail,
        )?;
        Ok(())
    }

    fn postfix_chain_overflow_last_unless_separate_lines_preferred(
        &self,
        overflowable: &PostfixItem<'_>,
        start_pos: u32,
        tail: &Tail,
    ) -> FormatResult {
        let first_line = self.out.line();
        let checkpoint = self.out.checkpoint();
        let enforce_max_width_guard = self.out.enforce_max_width();

        self.with_chain_item_max_width(start_pos, || self.postfix_overflowable(overflowable))?;
        // todo can we prove that the overflow is so long that a separate line won't be shorter?
        let overflow_height = self.out.line() - first_line + 1;
        self.tail(tail)?;
        if overflow_height == 1 {
            // it all fits on one line
            return Ok(());
        }
        drop(enforce_max_width_guard);
        let overflow_lookahead = self.out.capture_lookahead(&checkpoint);

        // try writing the overflowable on the next line to measure its height in separate lines format
        // todo share logic with match arm
        // todo should we check if the wrap allows a *longer* first line, like match arm?
        // todo account for having less width if the fallback will add a block
        let result = self.out.with_enforce_max_width(|| self
            .indented(|| {
                self.newline_break_indent()?;
                self.postfix_item(overflowable)?;
                let height = self.out.line() - first_line + 1;
                self.tail(tail)?;
                Ok(height)
            }))
            .constraint_err_only()?;

        match result {
            Err(_) => {
                self.out.restore_checkpoint(&checkpoint);
                self.out.restore_lookahead(&overflow_lookahead);
                Ok(())
            }
            Ok(separate_lines_height) => {
                if separate_lines_height <= overflow_height {
                    // fallback to separate lines strategy
                    Err(ConstraintErrorKind::NextStrategy.into())
                } else {
                    self.out.restore_checkpoint(&checkpoint);
                    self.out.restore_lookahead(&overflow_lookahead);
                    Ok(())
                }
            }
        }
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
            self.newline_break_indent()?;
            self.postfix_item(item)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn with_chain_item_max_width(
        &self,
        start_pos: u32,
        format: impl Fn() -> FormatResult,
    ) -> FormatResult {
        let offset = self.out.last_line_len() - start_pos;
        let limit = (offset >= POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH)
            .then_some(POSTFIX_CHAIN_MAX_WIDTH);
        self.with_width_limit_from_start_first_line_opt(start_pos, limit, format)
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
                self.path_segment(&method_call.seg, true, &self.tail_token("("))?;
                // todo this is consistent with rustfmt, but would it be better to force args to be
                //   on the same line, just allowing overflow of the last arg?
                self.call_args_after_open_paren(&method_call.args, Tail::none())?;
            }
            // root expression
            _ => self.expr(item)?,
        }
        Ok(())
    }

    fn postfix_items(&self, items: &[PostfixItem<'_>], start_pos: u32) -> FormatResult {
        items.iter().try_for_each(|item| {
            self.with_chain_item_max_width(start_pos, || self.postfix_item(item))
        })
    }

    fn postfix_tail(&self, tail: &[&ast::Expr]) -> FormatResult {
        for expr in tail {
            match expr.kind {
                ast::ExprKind::Index(_, ref index, _) => {
                    self.out.token("[")?;
                    self.backtrack()
                        .next(|| {
                            self.with_single_line(|| self.expr_tail(index, &self.tail_token("]")))
                        })
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
