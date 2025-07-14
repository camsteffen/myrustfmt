use crate::ast_formatter::list::options::ListStrategies;
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::enclosed::ENCLOSED_DISALLOWED_VSTRUCTS;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::ast_formatter::width_thresholds::WIDTH_THRESHOLDS;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::VSize;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::cell::Cell;

struct PostfixItem<'a> {
    /// The first item in the chain has the root expression, which is not a postfix expression.
    /// Subsequent items have a dot-notation item here, like `.field` or `.method()`.
    root_or_dot_item: &'a ast::Expr,
    /// any `?` or `[index]` occurring after `root_or_dot_item`
    postfix_tail: Vec<&'a ast::Expr>,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let chain = build_postfix_chain(expr);
        let mut chain = chain.as_slice();
        let (start_line, start_col) = self.out.line_col();

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.total_indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            let next = chain.split_off_first().unwrap();
            if chain.is_empty() {
                return self.postfix_item_tail(next, tail);
            }
            self.has_vstruct(VStruct::NonBlockIndent, || self.postfix_item(next))?;
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
            let width_limit_end =
                (chain.len() > 1).then(|| start_col.saturating_add(WIDTH_THRESHOLDS.chain_width));
            self.backtrack()
                .next(|_| {
                    let _guard = self.recover_width_guard();
                    let _guard = self.width_limit_end_opt_guard(width_limit_end)?;
                    self.postfix_chain_horizontal(chain, tail)
                })
                .next(|_| {
                    self.has_vstruct(VStruct::NonBlockIndent, || {
                        self.indented(|| self.postfix_chain_vertical(chain, tail))
                    })
                })
                .result()
        }
    }

    fn postfix_chain_horizontal(&self, chain: &[PostfixItem], tail: Tail) -> FormatResult {
        let wrappable_items = chain.len();
        let (last, before_last) = chain.split_last().unwrap();
        let items = |items: &[PostfixItem]| {
            let _guard = self.single_line_guard();
            items.iter().try_for_each(|item| self.postfix_item(item))
        };
        match &last.root_or_dot_item.kind {
            ast::ExprKind::MethodCall(method_call) => {
                items(before_last)?;
                self.postfix_chain_horizontal_last_method_call(
                    method_call,
                    last,
                    wrappable_items,
                    tail,
                )?;
            }
            // other postfix expression kinds are not overflowable
            _ => {
                items(chain)?;
                self.tail(tail)?;
            }
        }
        Ok(())
    }

    fn postfix_chain_horizontal_last_method_call(
        &self,
        method_call: &ast::MethodCall,
        postfix_item: &PostfixItem,
        wrappable_items: usize,
        tail: Tail,
    ) -> FormatResult {
        let method_col_start = self.out.col();
        let checkpoint = self.out.checkpoint();
        let path_wrap_result = self.simulate_wrap_indent(0, || {
            self.method_call_with_args_if_empty(method_call, postfix_item, tail)
        })?;
        match path_wrap_result {
            // the path fits in one line
            SimulateWrapResult::Ok => {
                if method_call.args.is_empty() {
                    return Ok(());
                }
            }

            // A vertical chain wouldn't prevent vertical path generics
            SimulateWrapResult::NoWrap => {
                self.out.restore_checkpoint(&checkpoint);
                self.method_call_with_args_if_empty(method_call, postfix_item, tail)?;
                if !method_call.args.is_empty() {
                    self.method_call_args_postfix_tail(
                        method_call,
                        ListStrategies::flexible_overflow(),
                        postfix_item,
                        tail,
                    )?;
                }
                return Ok(());
            }

            // Prefer a vertical chain to avoid vertical path generics
            SimulateWrapResult::WrapForSingleLine
            | SimulateWrapResult::WrapForLongerFirstLine
            | SimulateWrapResult::WrapForLessExcessWidth => {
                return Err(self.err(FormatErrorKind::Logical));
            }
        }

        // You are here: x.method(args)?
        //                        ^

        let checkpoint = self.out.checkpoint();

        // Simulate wrapping the method call as if in a vertical chain.
        let width_before_args = self.out.col() - method_col_start;
        let wrap_result = self.simulate_wrap_indent(width_before_args, || {
            self.method_call_args_postfix_tail(
                method_call,
                ListStrategies::flexible_overflow(),
                postfix_item,
                tail,
            )?;
            Ok(())
        })?;

        let args_horizontal_or_recover = || {
            self.out.restore_checkpoint(&checkpoint);
            let result = self.method_call_args_postfix_tail(
                method_call,
                ListStrategies::horizontal_overflow(),
                postfix_item,
                tail,
            );
            match result {
                Ok(height) => Ok(Some(height)),
                Err(e) => {
                    // recover errors that might be avoided with vertical args or a vertical chain
                    let recovering = match e.kind {
                        FormatErrorKind::WidthLimitExceeded => true,
                        FormatErrorKind::Vertical(_) => true,
                        FormatErrorKind::VStruct { vstruct, .. } => {
                            !ENCLOSED_DISALLOWED_VSTRUCTS.contains(vstruct)
                        }
                        _ => false,
                    };
                    if recovering { Ok(None) } else { Err(e) }
                }
            }
        };

        let vertical_args = || {
            self.out.restore_checkpoint(&checkpoint);
            self.method_call_args_postfix_tail(
                method_call,
                ListStrategies::vertical(),
                postfix_item,
                tail,
            )?;
            Ok(())
        };

        match wrap_result {
            SimulateWrapResult::Ok => Ok(()),

            // The method call fits in one line if we format the chain vertically
            SimulateWrapResult::WrapForSingleLine => {
                // At this point we can calculate the height of a vertical chain
                let vertical_chain_height = 1 + wrappable_items as VSize;
                if let Some(horizontal_args_height) = args_horizontal_or_recover()? {
                    if vertical_chain_height <= horizontal_args_height {
                        // A vertical chain is at least as short as a horizontal chain with overflow
                        return Err(self.err(FormatErrorKind::Logical));
                    }
                    // Use a horizontal chain with horizontal method call arguments with overflow
                    Ok(())
                } else {
                    let vertical_args_height = (2 + method_call.args.len()) as VSize;
                    if vertical_chain_height <= vertical_args_height {
                        return Err(self.err(FormatErrorKind::Logical));
                    }
                    // Use a horizontal chain with vertical method call arguments.
                    vertical_args()?;
                    Ok(())
                }
            }

            // No reason not to use horizontal method call arguments
            SimulateWrapResult::NoWrap
            // A longer first line in the overflow isn't enough to prefer a vertical chain when we
            // can do a horizontal chain with horizontal method call arguments. A vertical chain
            // would increase the indentation level of the overflowed content.
            | SimulateWrapResult::WrapForLongerFirstLine => {
                if args_horizontal_or_recover()?.is_some() {
                    Ok(())
                } else if wrap_result == SimulateWrapResult::WrapForLongerFirstLine
                    && wrappable_items <= method_call.args.len() + 1
                {
                    // We have a choice between vertical method call arguments or a vertical chain.
                    // The last method call argument overflows in either case, and the indentation level
                    // within the overflow is the same. A vertical chain is preferred if it is the same
                    // height or shorter.
                    Err(self.err(FormatErrorKind::Logical))
                } else {
                    // At this point we want vertical method call arguments if possible
                    vertical_args()?;
                    Ok(())
                }
            }

            // Use a vertical chain if it has less excess width.
            SimulateWrapResult::WrapForLessExcessWidth => Err(self.err(FormatErrorKind::Logical)),
        }
    }

    fn postfix_chain_vertical(&self, chain: &[PostfixItem], tail: Tail) -> FormatResult {
        for item in chain {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.postfix_item(item)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn postfix_item(&self, item: &PostfixItem) -> FormatResult {
        self.postfix_item_tail(item, None)
    }

    fn postfix_item_tail(&self, item: &PostfixItem, tail: Tail) -> FormatResult {
        let postfix_tail = |af: &Self| af.postfix_tail(&item.postfix_tail, tail);
        match item.root_or_dot_item.kind {
            ast::ExprKind::Await(..) => {
                self.out.token(".")?;
                self.out.token("await")?;
                postfix_tail(self)?;
            }
            ast::ExprKind::Field(_, ident) => {
                self.out.token(".")?;
                self.ident(ident)?;
                postfix_tail(self)?;
            }
            ast::ExprKind::MethodCall(ref method_call) => {
                self.out.token(".")?;
                self.path_segment(
                    &method_call.seg,
                    true,
                    Some(&self.tail_fn(|af| {
                        af.out.token("(")?;
                        af.method_call_args_postfix_tail(
                            method_call,
                            ListStrategies::flexible_overflow(),
                            item,
                            tail,
                        )?;
                        Ok(())
                    })),
                )?;
            }
            // root expression
            _ => {
                self.expr_tail(item.root_or_dot_item, Some(&self.tail_fn(postfix_tail)))?;
            }
        }
        Ok(())
    }

    fn method_call_with_args_if_empty(
        &self,
        method_call: &ast::MethodCall,
        postfix_item: &PostfixItem,
        tail: Tail,
    ) -> FormatResult {
        self.out.token(".")?;
        self.path_segment(
            &method_call.seg,
            true,
            Some(&self.tail_fn(|af| {
                af.out.token("(")?;
                if method_call.args.is_empty() {
                    af.method_call_args_postfix_tail(
                        method_call,
                        ListStrategies::horizontal_overflow(),
                        postfix_item,
                        tail,
                    )?;
                }
                Ok(())
            })),
        )?;
        Ok(())
    }

    // x.method(args)[0]?
    //          ‾‾‾‾‾‾‾‾‾
    fn method_call_args_postfix_tail(
        &self,
        method_call: &ast::MethodCall,
        list_strategies: ListStrategies<P<ast::Expr>>,
        postfix_item: &PostfixItem,
        tail: Tail,
    ) -> FormatResult<VSize> {
        let first_line = self.out.line();
        let height = Cell::new(0);
        self.call_args(
            &method_call.args,
            list_strategies,
            Some(&self.tail_fn(|af| {
                height.set(af.out.line() - first_line + 1);
                af.postfix_tail(&postfix_item.postfix_tail, tail)
            })),
        )?;
        Ok(height.get())
    }

    fn postfix_tail(&self, postfix_tail: &[&ast::Expr], tail: Tail) -> FormatResult {
        if postfix_tail.is_empty() {
            return self.tail(tail);
        }
        for (i, expr) in postfix_tail.iter().enumerate() {
            let is_last = i == postfix_tail.len() - 1;
            let tail = if is_last { tail } else { None };
            match expr.kind {
                ast::ExprKind::Index(_, ref index, _) => {
                    self.out.token("[")?;
                    self.backtrack()
                        .next(|_| {
                            let _guard = self.single_line_guard();
                            self.expr(index)?;
                            self.out.token("]")?;
                            self.tail(tail)?;
                            Ok(())
                        })
                        .next(|_| {
                            self.has_vstruct(VStruct::Index, || {
                                self.enclosed_contents(|| self.expr(index))?;
                                self.out.token("]")?;
                                self.tail(tail)?;
                                Ok(())
                            })
                        })
                        .result()?;
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

fn build_postfix_chain(expr: &ast::Expr) -> Vec<PostfixItem<'_>> {
    let mut current = expr;
    let mut items = Vec::new();
    let mut postfix_tail = Vec::new();
    let root = loop {
        if postfix_expr_is_dot(current) {
            items.push(PostfixItem {
                root_or_dot_item: current,
                postfix_tail: postfix_tail.drain(..).rev().collect(),
            })
        } else {
            postfix_tail.push(current);
        }
        let receiver = postfix_expr_receiver(current);
        if !is_postfix_expr(receiver) {
            break receiver;
        }
        current = receiver;
    };
    postfix_tail.reverse();
    items.push(PostfixItem {
        root_or_dot_item: root,
        postfix_tail,
    });
    items.reverse();
    items
}
