use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::error::{FormatError, WidthLimitExceededError};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use std::iter::successors;
use crate::ast_utils::{postfix_expr_is_breakable, postfix_expr_receiver};

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
        self.postfix_chain_items(take_unbreakables(&mut chain_rest))?;

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            if self.out.line() != first_line {
                break true;
            }
            if self.out.last_line_len() > indent_margin {
                break false;
            }
            let Some(next) = take_next_with_unbreakables(&mut chain_rest) else {
                return self.tail(tail);
            };
            self.postfix_chain_items(next)?
        };

        if multi_line_root {
            // each item on a separate line, no indent
            self.postfix_chain_separate_lines(chain_rest, tail)
        } else {
            self.fallback(|| self.postfix_chain_single_line(chain_rest, start_pos, tail))
                .otherwise(|| self.indented(|| self.postfix_chain_separate_lines(chain_rest, tail)))
        }
    }

    fn postfix_chain_single_line(
        &self,
        chain: &[&ast::Expr],
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

        // first try without overflow
        let result = items_single_line(overflowable).and_then(|()| self.tail(tail));
        match result {
            Ok(()) | Err(FormatError::Parse(_)) => return result,
            Err(FormatError::Constraint(_)) => {
                self.out.restore(&snapshot);
            }
        }

        // experimentally check if wrapping the last item makes it fit on one line
        // the width limit would not apply
        // todo wouldn't the first line width limit be removed anyways from the newline?
        // todo is this over-complicated?
        let result = self.indented(|| {
            self.out.newline_within_indent()?;
            self.with_single_line(|| {
                self.postfix_chain_items(overflowable)?;
                self.tail(tail)?;
                FormatResult::Ok(())
            })?;
            Ok(())
        });
        if result.is_ok() {
            // ...if so, go to the separate lines approach
            return Err(WidthLimitExceededError.into());
        }
        self.out.restore(&snapshot);

        // finally, use overflow
        self.with_width_limit_from_start_first_line(start_pos, width_limit, || {
            let (first, rest) = overflowable.split_first().unwrap();
            self.postfix_chain_item(first)?;
            self.with_single_line(|| self.postfix_chain_items(rest))?;
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn postfix_chain_separate_lines(&self, mut chain: &[&ast::Expr], tail: &Tail) -> FormatResult {
        while let Some(next) = take_next_with_unbreakables(&mut chain) {
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
                self.fallback(|| {
                    self.with_single_line(|| self.expr_tail(index, &Tail::token("]")))
                })
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
    let mut chain = Vec::from_iter(successors(Some(expr), |e| postfix_expr_receiver(e)));
    // Separate the root because
    //   1) It allows the chain to be *only* postfix expressions
    //   2) It puts this unwrap close to the source of the never-empty list
    let root = chain.pop().unwrap();
    // order the chain as it appears in code
    chain.reverse();
    (root, chain)
}

/// Splits of the trailing method call, with optional unbreakable items after that,
/// or returns None if the chain doesn't end with a method call.
fn split_overflowable<'a, 'b>(
    chain: &'b [&'a ast::Expr],
) -> Option<(&'b [&'a ast::Expr], &'b [&'a ast::Expr])> {
    let mut count = 0;
    for item in chain.iter().rev() {
        if is_unbreakable(item) {
            count += 1;
        } else if matches!(item.kind, ast::ExprKind::MethodCall(_)) {
            count += 1;
            return Some(chain.split_at(chain.len() - count));
        } else {
            break;
        }
    }
    None
}

fn is_unbreakable(item: &ast::Expr) -> bool {
    !postfix_expr_is_breakable(item).unwrap()
}

fn take_next_with_unbreakables<'a, 'b>(
    slice: &mut &'a [&'b ast::Expr],
) -> Option<&'a [&'b ast::Expr]> {
    match slice {
        [] => None,
        [_, rest @ ..] => {
            let pos = rest.iter().take_while(|e| is_unbreakable(e)).count();
            let out;
            (out, *slice) = slice.split_at(pos + 1);
            Some(out)
        }
    }
}

fn take_unbreakables<'a, 'b>(slice: &mut &'b [&'a ast::Expr]) -> &'b [&'a ast::Expr] {
    let pos = slice.iter().take_while(|e| is_unbreakable(e)).count();
    let out;
    (out, *slice) = slice.split_at(pos);
    out
}
