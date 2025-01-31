use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{postfix_expr_is_wrappable, postfix_expr_receiver_opt};
use crate::error::FormatResult;
use crate::error::WidthLimitExceededError;
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
            self.postfix_chain_single_line(&chain, start_pos, tail)?;
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
                .next(|| self.postfix_chain_single_line(chain_rest, start_pos, tail))
                .otherwise(|| self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail)))
        }
    }

    fn postfix_chain_single_line(
        &self,
        chain: &[&ast::Expr],
        start_pos: u32,
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

        let mut backtrack = self.backtrack();

        // todo break this up more cleanly, avoid needless backtrack
        if !self.constraints().touchy_margin.get() {
            // first try without overflow
            backtrack = backtrack.next(|| {
                items_single_line(overflowable)?;
                self.tail(tail)?;
                Ok(())
            });

            let result = backtrack.next_control_flow(|| {
                // experimentally check if wrapping the last item makes it fit on one line
                // the width limit would not apply
                // todo wouldn't the first line width limit be removed anyways from the newline?
                // todo is this over-complicated?
                // todo share logic with match arm
                let fits_on_one_line_with_wrap = self
                    .indented(|| {
                        self.out.newline_within_indent()?;
                        self.with_single_line(|| {
                            self.postfix_chain_items(overflowable)?;
                            self.tail(tail)?;
                            FormatResult::Ok(())
                        })?;
                        Ok(())
                    })
                    .is_ok();
                if fits_on_one_line_with_wrap {
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
                    // fragment, probe, segment, lookahead
                    let wrappable_count = until_overflow.iter().filter(|e| postfix_expr_is_wrappable(e)).count();
                    ControlFlow::Break(Err(WidthLimitExceededError.into()))
                } else {
                    ControlFlow::Continue(())
                }
            });
            backtrack = match result {
                ControlFlow::Break(result) => return result,
                ControlFlow::Continue((backtrack, ())) => backtrack,
            };
        }

        backtrack.otherwise(|| {
            // finally, use overflow
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
            })?;
            self.tail(tail)?;
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

/// Splits off the trailing method call, possibly followed by non-wrappable items,
/// or returns None if the chain doesn't end with a method call.
fn split_overflowable<'a, 'b>(
    chain: &'b [&'a ast::Expr],
) -> Option<(&'b [&'a ast::Expr], &'b [&'a ast::Expr])> {
    let mut len = 1;
    for item in chain.iter().rev() {
        if matches!(item.kind, ast::ExprKind::MethodCall(_)) {
            return Some(chain.split_at(chain.len() - len));
        } else if postfix_expr_is_wrappable(item) {
            return None;
        } else {
            len += 1;
        }
    }
    None
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
