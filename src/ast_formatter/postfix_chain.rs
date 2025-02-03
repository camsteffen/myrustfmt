use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{postfix_expr_is_wrappable, postfix_expr_receiver_opt};
use crate::error::{ConstraintError, FormatError, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use std::iter::successors;
use std::ops::ControlFlow;

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let (root, chain) = build_postfix_chain(expr);

        let first_line = self.out.line();
        let start_pos = self.out.last_line_len();

        // touchy margins - everything must be a single line (with overflow)
        if self.out.constraints().touchy_margin.get() {
            self.with_single_line(|| self.expr(root))?;
            self.postfix_chain_single_line(&chain, start_pos, tail, false)?;
            return Ok(());
        }

        self.expr(root)?;
        let mut chain_rest = chain.as_slice();
        self.postfix_chain_items(take_non_wrappables(&mut chain_rest))?;

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            if self.out.line() != first_line {
                break true;
            }
            if self.out.last_line_len() > indent_margin {
                break false;
            }
            let Some(next) = take_next_with_non_wrappables(&mut chain_rest) else {
                return self.tail(tail);
            };
            self.postfix_chain_items(next)?
        };

        if multi_line_root {
            // each item on a separate line, no indent
            self.postfix_chain_separate_lines(chain_rest, tail)
        } else {
            self.backtrack()
                .next(|| self.postfix_chain_single_line(chain_rest, start_pos, tail, true))
                .otherwise(|| self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail)))
        }
    }

    fn postfix_chain_single_line(
        &self,
        chain: &[&ast::Expr],
        start_pos: u32,
        tail: &Tail,
        // todo factor out bool?
        has_separate_lines_fallback: bool,
    ) -> FormatResult {
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        let items_single_line = |chain| {
            self.with_width_limit_from_start(start_pos, width_limit, || {
                self.with_single_line(|| self.postfix_chain_items(chain))
            })
        };

        let Some((until_overflow, overflowable)) = split_overflowable(chain) else {
            // this chain is not overflowable, so simply format it on one line
            items_single_line(chain)?;
            self.tail(tail)?;
            return Ok(());
        };

        items_single_line(until_overflow)?;

        if !has_separate_lines_fallback {
            self.postfix_overflowable(overflowable, start_pos)?;
            self.tail(tail)?;
            return Ok(());
        }

        let wrappable_count = until_overflow
            .iter()
            .filter(|e| postfix_expr_is_wrappable(e))
            .count() as u32;
        self.postfix_overflowable_unless_separate_lines_is_better(
            overflowable,
            start_pos,
            tail,
            wrappable_count,
        )?;
        Ok(())
    }

    fn postfix_overflowable_unless_separate_lines_is_better(
        &self,
        overflowable: &[&ast::Expr],
        start_pos: u32,
        tail: &Tail,
        wrappable_count: u32,
    ) -> FormatResult {
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        let items_single_line = |chain| {
            self.with_width_limit_from_start(start_pos, width_limit, || {
                self.with_single_line(|| self.postfix_chain_items(chain))
            })
        };

        // first try writing in one line without overflow
        let backtrack = self.backtrack().next(|| {
            items_single_line(overflowable)?;
            self.tail(tail)?;
            Ok(())
        });

        // try writing the overflowable on the next line to measure its height in separate lines format
        let result = backtrack
            .next_control_flow(|| {
                // todo can we use first line width limit and rely on the newline here to disable it?
                // todo share logic with match arm
                // todo should we check if the wrap allows a *longer* first line, like match arm?
                let result = self.indented(|| {
                    self.out.newline_within_indent()?;
                    let line = self.out.line();
                    self.postfix_chain_items(overflowable)?;
                    self.tail(tail)?;
                    Ok(self.out.line() - line + 1)
                });
                match result {
                    Err(FormatError::Constraint(_)) => ControlFlow::Continue(None),
                    Err(e) => ControlFlow::Break(Err(e)),
                    Ok(height) => ControlFlow::Continue(Some(height)),
                }
                // ...if so, go to the separate lines approach
                // todo perhaps there is a concept of "fallback cost" that can be passed down
                //   - if you err out, there will be a cost of increased indent and added lines
                /* todo
                    1. Count total number of lines when deciding to fall back to multi-line chain
                       - we can count the number of wrappables leading up to the method call, and
                         we know they will take one line each since they all fit on one line
                    2. Consider touchy_margins
                       - Consider 2 lines added by block
                       - Consider increased indentation
                         - less room for wrapped method call
                         - less room for items leading up to method call, so multi-line may already be a given
                */
            });
        let (backtrack, overflowable_separate_line_height) = match result {
            ControlFlow::Break(result) => return result,
            ControlFlow::Continue(c) => c,
        };

        // finally, use overflow, but reject it if separate lines is preferable
        backtrack.otherwise(|| {
            let first_line = self.out.line();
            self.postfix_overflowable(overflowable, start_pos)?;
            self.tail(tail)?;
            let overflow_height = self.out.line() - first_line;
            if let Some(overflowable_separate_line_height) = overflowable_separate_line_height {
                // Each wrappable leading up to the overflowable will occupy one line each since we were
                // previously able to format them all on one line.
                let separate_lines_height = wrappable_count + overflowable_separate_line_height;
                if separate_lines_height <= overflow_height {
                    // fallback to separate lines strategy
                    return Err(ConstraintError::Logical.into());
                }
            }
            // keep overflow format
            Ok(())
        })
    }

    fn postfix_overflowable(&self, overflowable: &[&ast::Expr], start_pos: u32) -> FormatResult {
        let width_limit = RUSTFMT_CONFIG_DEFAULTS.chain_width;
        self.with_width_limit_from_start_first_line(start_pos, width_limit, || {
            let (first, rest) = overflowable.split_first().unwrap();
            self.postfix_chain_item(first)?;
            // single line constraint here avoids cascading overflow along the margin like:
            // chain.method({
            //     expr
            // })[
            //     expr
            // ]
            self.with_single_line(|| self.postfix_chain_items(rest))?;
            Ok(())
        })
    }

    fn postfix_chain_separate_lines(&self, mut chain: &[&ast::Expr], tail: &Tail) -> FormatResult {
        while let Some(next) = take_next_with_non_wrappables(&mut chain) {
            self.out.newline_within_indent()?;
            self.postfix_chain_items(next)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn postfix_chain_item(&self, item: &ast::Expr) -> FormatResult {
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
                self.call_args_after_open_paren(&method_call.args, Tail::none())
                    .format(self)?;
            }
            ast::ExprKind::Index(_, ref index, _) => {
                self.out.token("[")?;
                self.backtrack()
                    .next(|| self.with_single_line(|| self.expr_tail(index, &Tail::token("]"))))
                    .otherwise(|| self.embraced_after_opening("]", || self.expr(index)))?;
            }
            ast::ExprKind::Try(..) => self.out.token("?")?,
            _ => unreachable!(),
        }
        Ok(())
    }

    fn postfix_chain_items(&self, items: &[&ast::Expr]) -> FormatResult {
        items
            .iter()
            .try_for_each(|item| self.postfix_chain_item(item))
    }
}

fn build_postfix_chain(expr: &ast::Expr) -> (&ast::Expr, Vec<&ast::Expr>) {
    let mut chain = Vec::from_iter(successors(Some(expr), |e| postfix_expr_receiver_opt(e)));
    let root = chain.pop().unwrap();
    // order the chain as it appears in code
    chain.reverse();
    // N.B. `root` can be any expression type, but `chain` is strictly postfix expressions
    (root, chain)
}

/// Splits off the trailing method call, possibly followed by non-wrappable items.
/// Returns None if the chain doesn't end with a method call.
fn split_overflowable<'a, 'b>(
    chain: &'b [&'a ast::Expr],
) -> Option<(&'b [&'a ast::Expr], &'b [&'a ast::Expr])> {
    chain
        .iter()
        // get the last wrappable item
        .rposition(|e| postfix_expr_is_wrappable(e))
        // that item must be a method call to be "overflowable"
        .filter(|&i| matches!(chain[i].kind, ast::ExprKind::MethodCall(_)))
        .map(|i| chain.split_at(i))
}

fn take_next_with_non_wrappables<'a, 'b>(
    slice: &mut &'a [&'b ast::Expr],
) -> Option<&'a [&'b ast::Expr]> {
    let (_, rest) = slice.split_first()?;
    let (out, new_slice) = slice.split_at(count_leading_non_wrappables(rest) + 1);
    *slice = new_slice;
    Some(out)
}

fn take_non_wrappables<'a, 'b>(slice: &mut &'b [&'a ast::Expr]) -> &'b [&'a ast::Expr] {
    let (out, new_slice) = slice.split_at(count_leading_non_wrappables(slice));
    *slice = new_slice;
    out
}

fn count_leading_non_wrappables(slice: &[&ast::Expr]) -> usize {
    slice
        .iter()
        .take_while(|e| !postfix_expr_is_wrappable(e))
        .count()
}
