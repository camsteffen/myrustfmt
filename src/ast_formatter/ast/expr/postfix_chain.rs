use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::constraints::Shape;
use crate::error::{ConstraintErrorKind, FormatResult};
use crate::num::HSize;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

// In rustfmt, this is called chain_width, and is 60 by default
const POSTFIX_CHAIN_MAX_WIDTH: HSize = 60;
/// Don't apply chain max width unless the chain item's distance from the start
/// of the chain is at least this much.
const POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH: HSize = 15;

struct PostfixItem<'a> {
    /// The first item in the chain has the root expression, which is not a postfix expression.
    /// Subsequent items have a dot-notation item here, like `.field` or `.method()`.
    root_or_dot_item: &'a ast::Expr,
    /// any `?` or `[index]` occurring after `root_or_dot_item`
    non_dot_items: Vec<&'a ast::Expr>,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let chain = build_postfix_chain(expr);
        let mut chain = chain.as_slice();
        let (start_line, start_col) = self.out.line_col();

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.total_indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            // let line_before_comments = self.out.line();
            // self.out.comments(VerticalWhitespaceMode::Break)?;
            // if self.out.line() != first_line {
            //     // todo test
            //     break line_before_comments != first_line;
            // }
            let next = chain.split_off_first().unwrap();
            if chain.is_empty() {
                return self.postfix_item_tail(next, tail, false);
            }
            self.has_shape(Shape::Any, || self.postfix_item(next))?;
            if self.out.line() != start_line {
                break true;
            }
            if self.out.col() > indent_margin {
                break false;
            }
        };

        if multi_line_root {
            // no indent
            self.postfix_chain_vertical(chain, tail)
        } else {
            self.backtrack()
                .next(|| self.postfix_chain_single_line_with_overflow(chain, start_col, tail))
                .otherwise(|| {
                    self.has_shape(Shape::HangingIndent, || {
                        self.indented(|| self.postfix_chain_vertical(chain, tail))
                    })
                })
        }
    }

    fn postfix_chain_single_line_with_overflow(
        &self,
        chain: &[PostfixItem],
        start_col: HSize,
        tail: Tail,
    ) -> FormatResult {
        let last = chain.last().unwrap();
        if !matches!(last.root_or_dot_item.kind, ast::ExprKind::MethodCall(_)) {
            // this chain is not overflowable, so simply format it on one line
            self.with_single_line(|| self.postfix_items(chain, start_col))?;
            self.tail(tail)?;
            return Ok(());
        }
        let (overflowable, before_overflow) = chain.split_last().unwrap();
        self.with_single_line(|| self.postfix_items(before_overflow, start_col))?;
        self.postfix_chain_overflow(overflowable, start_col, tail)?;
        Ok(())
    }

    fn postfix_chain_overflow(
        &self,
        overflowable: &PostfixItem,
        start_col: HSize,
        tail: Tail,
    ) -> FormatResult {
        let first_line = self.out.line();
        let checkpoint = self.out.checkpoint();
        let overflow_height = self.out.with_recoverable_width(|| -> FormatResult<_> {
            self.with_chain_item_max_width(start_col, || {
                self.postfix_item_tail(overflowable, &None, true)
            })?;
            // todo can we prove that the overflow is so long that a separate line won't be shorter?
            let overflow_height = self.out.line() - first_line + 1;
            self.tail(tail)?;
            Ok(overflow_height)
        })?;
        if overflow_height == 1 {
            // it all fits on one line
            return Ok(());
        }
        let overflow_lookahead = self.out.capture_lookahead(&checkpoint);

        // try writing the overflowable on the next line to measure its height in separate lines format
        // todo share logic with match arm
        // todo should we check if the wrap allows a *longer* first line, like match arm?
        // todo account for having less width if the fallback will add a block
        let result = self.out.with_recoverable_width(|| {
            self.indented(|| {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                self.postfix_item(overflowable)?;
                let height = self.out.line() - first_line + 1;
                self.tail(tail)?;
                Ok(height)
            })
        });

        match result {
            Err(_) => {
                self.out.restore_checkpoint(&checkpoint);
                self.out.restore_lookahead(overflow_lookahead);
                Ok(())
            }
            Ok(vertical_height) => {
                if vertical_height <= overflow_height {
                    // use vertical strategy
                    Err(ConstraintErrorKind::NextStrategy.into())
                } else {
                    self.out.restore_checkpoint(&checkpoint);
                    self.out.restore_lookahead(overflow_lookahead);
                    Ok(())
                }
            }
        }
        // todo perhaps there is a concept of "fallback cost" that can be passed down
        //   - if you err out, there will be a cost of increased indent and added lines
    }

    fn postfix_chain_vertical(&self, chain: &[PostfixItem], tail: Tail) -> FormatResult {
        for item in chain {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.postfix_item(item)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn with_chain_item_max_width(
        &self,
        start_col: HSize,
        format: impl Fn() -> FormatResult,
    ) -> FormatResult {
        let offset = self.out.col() - start_col;
        let limit = (offset >= POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH)
            .then_some(POSTFIX_CHAIN_MAX_WIDTH);
        self.with_width_limit_from_start_first_line_opt(start_col, limit, format)
    }

    fn postfix_item(&self, item: &PostfixItem) -> FormatResult {
        self.postfix_item_tail(item, &None, false)
    }

    fn postfix_item_tail(
        &self,
        item: &PostfixItem,
        tail: Tail,
        single_line_tail: bool,
    ) -> FormatResult {
        let non_dot_items = |af: &Self| {
            af.with_single_line_opt(single_line_tail, || {
                af.postfix_non_dot_items(&item.non_dot_items, tail)
            })
        };
        match item.root_or_dot_item.kind {
            ast::ExprKind::Await(..) => {
                self.out.token(".")?;
                self.out.token("await")?;
                non_dot_items(self)?;
            }
            ast::ExprKind::Field(_, ident) => {
                self.out.token(".")?;
                self.ident(ident)?;
                non_dot_items(self)?;
            }
            ast::ExprKind::MethodCall(ref method_call) => {
                self.out.token(".")?;
                self.path_segment(&method_call.seg, true, &self.tail_token("("))?;
                // todo this is consistent with rustfmt, but would it be better to force args to be
                //   on the same line, just allowing overflow of the last arg?
                self.call_args_after_open_paren(&method_call.args, &self.tail_fn(non_dot_items))?;
            }
            // root expression
            _ => {
                self.expr_tail(item.root_or_dot_item, &self.tail_fn(non_dot_items))?;
            }
        }
        Ok(())
    }

    fn postfix_items(&self, items: &[PostfixItem], start_col: HSize) -> FormatResult {
        items.iter().try_for_each(|item| {
            self.with_chain_item_max_width(start_col, || self.postfix_item(item))
        })
    }

    fn postfix_non_dot_items(&self, postfix_tail: &[&ast::Expr], tail: Tail) -> FormatResult {
        if postfix_tail.is_empty() {
            return self.tail(tail);
        }
        for (i, expr) in postfix_tail.iter().enumerate() {
            let is_last = i == postfix_tail.len() - 1;
            let tail = if is_last { tail } else { &None };
            match expr.kind {
                ast::ExprKind::Index(_, ref index, _) => {
                    self.out.token("[")?;
                    self.backtrack()
                        .next(|| {
                            self.with_single_line(|| {
                                self.expr(index)?;
                                self.out.token("]")?;
                                self.tail(tail)?;
                                Ok(())
                            })
                        })
                        .otherwise(|| {
                            self.enclosed_after_opening("]", || self.expr(index))?;
                            self.tail(tail)?;
                            Ok(())
                        })?;
                }
                ast::ExprKind::Try(..) => {
                    self.out.token("?")?;
                    self.tail(tail)?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

fn build_postfix_chain(expr: &ast::Expr) -> Vec<PostfixItem> {
    let mut current = expr;
    let mut items = Vec::new();
    let mut non_dot_items = Vec::new();
    let root = loop {
        if postfix_expr_is_dot(current) {
            items.push(PostfixItem {
                root_or_dot_item: current,
                non_dot_items: non_dot_items.drain(..).rev().collect(),
            })
        } else {
            non_dot_items.push(current);
        }
        let receiver = postfix_expr_receiver(current);
        if !is_postfix_expr(receiver) {
            break receiver;
        }
        current = receiver;
    };
    non_dot_items.reverse();
    items.push(PostfixItem {
        root_or_dot_item: root,
        non_dot_items,
    });
    items.reverse();
    items
}
