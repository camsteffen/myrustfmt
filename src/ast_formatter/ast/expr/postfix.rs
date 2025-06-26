use crate::ast_formatter::list::options::ListStrategies;
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::{HSize, VSize};
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::cell::Cell;

// In rustfmt, this is called chain_width, and is 60 by default
const POSTFIX_CHAIN_MAX_WIDTH: HSize = 60;

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
            let width_limit_end = (chain.len() > 1).then(|| start_col + POSTFIX_CHAIN_MAX_WIDTH);
            self.backtrack()
                .next(|| {
                    self.out.with_recover_width(|| {
                        self.postfix_chain_horizontal(chain, width_limit_end, tail)
                    })
                })
                .next(|| {
                    self.has_vstruct(VStruct::NonBlockIndent, || {
                        self.indented(|| self.postfix_chain_vertical(chain, tail))
                    })
                })
                .result()
        }
    }

    fn postfix_chain_horizontal(
        &self,
        chain: &[PostfixItem],
        width_limit_end: Option<HSize>,
        tail: Tail,
    ) -> FormatResult {
        let wrappable_items = chain.len();
        let (last, before_last) = chain.split_last().unwrap();
        let items = |items: &[PostfixItem]| {
            self.with_single_line(|| {
                items.iter().try_for_each(|item| {
                    self.with_width_limit_end_opt(width_limit_end, || self.postfix_item(item))
                })
            })
        };
        match &last.root_or_dot_item.kind {
            ast::ExprKind::MethodCall(method_call) => {
                items(before_last)?;
                self.postfix_chain_horizontal_last_method_call(
                    method_call,
                    last,
                    width_limit_end,
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
        width_limit_end: Option<HSize>,
        wrappable_items: usize,
        tail: Tail,
    ) -> FormatResult {
        let method_col_start = self.out.col();
        let result = self.with_width_limit_end_opt(width_limit_end, || {
            // todo single line?
            self.method_call_dot_path(method_call)?;
            if method_call.args.is_empty() {
                self.method_call_args_postfix_tail(
                    method_call,
                    ListStrategies::horizontal_overflow(),
                    postfix_item,
                    tail,
                )?;
                Ok(None)
            } else {
                let checkpoint = self.out.checkpoint();

                // todo do horizontal args on-demand as the result is needed below
                // First just try to format the method call horizontally.
                let horizontal_args_result = self.method_call_args_postfix_tail(
                    method_call,
                    ListStrategies::horizontal_overflow(),
                    postfix_item,
                    tail,
                );
                Ok(Some((checkpoint, horizontal_args_result)))
            }
        })?;
        let Some((checkpoint, horizontal_args_result)) = result else {
            return Ok(());
        };

        let horizontal_args = match horizontal_args_result {
            // If it fits in one line, we're done!
            Ok(1) => return Ok(()),
            Ok(height) => {
                // It overflowed.
                let lookahead = self.out.capture_lookahead(&checkpoint);
                Some((height, lookahead))
            }
            Err(e) if let FormatErrorKind::UnsupportedSyntax = e.kind => return Err(e),
            Err(_) => {
                self.out.restore_checkpoint(&checkpoint);
                None
            }
        };

        // todo what if path segment is multi-line?
        let width_before_args = self.out.col() - method_col_start;

        // We may or may not have been able to format the method call horizontally. Either way,
        // next we'll simulate wrapping the method call as if in a vertical chain.
        let wrap_result = self.with_width_limit_end_opt(width_limit_end, || {
            Ok(self.simulate_wrap_indent(width_before_args, || {
                self.method_call_args_postfix_tail(
                    method_call,
                    ListStrategies::flexible_overflow(),
                    postfix_item,
                    tail,
                )?;
                Ok(())
            }))
        })?;

        // This is a very complex decision tree! One unifying theme is that vertical chains are
        // preferred when they do not increase the overall height. The principle is to prefer
        // splitting outer structures over inner structures.
        match wrap_result {
            // If it fits in one line normally, we should have early returned above.
            SimulateWrapResult::Ok => {
                panic!("chain simulate_wrap_indent should not fit in one line")
            }

            // The method call fits in one line if we format the chain vertically
            SimulateWrapResult::WrapForSingleLine => {
                // At this point we can calculate the height of a vertical chain
                let vertical_chain_height = 1 + wrappable_items as VSize;
                if let Some((horizontal_args_height, horizontal_args_lookahead)) = horizontal_args {
                    if vertical_chain_height <= horizontal_args_height {
                        // A vertical chain is at least as short as a horizontal chain with overflow
                        return Err(FormatErrorKind::Logical.into());
                    }
                    // Use a horizontal chain with horizontal method call arguments with overflow
                    self.out.restore_lookahead(horizontal_args_lookahead);
                } else {
                    let vertical_args_height = (2 + method_call.args.len()) as VSize;
                    if vertical_chain_height <= vertical_args_height {
                        return Err(FormatErrorKind::Logical.into());
                    }
                    // Use a horizontal chain with vertical method call arguments.
                    self.out.restore_checkpoint(&checkpoint);
                    self.method_call_args_postfix_tail(
                        method_call,
                        ListStrategies::vertical(),
                        postfix_item,
                        tail,
                    )?;
                }
            }

            // No reason not to use horizontal method call arguments
            SimulateWrapResult::NoWrap
            // A longer first line in the overflow isn't enough to prefer a vertical chain when we
            // can do a horizontal chain with horizontal method call arguments. A vertical chain
            // would increase the indentation level of the overflowed content.
            | SimulateWrapResult::WrapForLongerFirstLine
                if let Some((_, horizontal_args_lookahead)) = horizontal_args =>
            {
                self.out.restore_lookahead(horizontal_args_lookahead);
            }

            // We have a choice between vertical method call arguments or a vertical chain.
            // The last method call argument overflows in either case, and the indentation level
            // within the overflow is the same. A vertical chain is preferred if it is the same
            // height or shorter.
            SimulateWrapResult::WrapForLongerFirstLine
                if wrappable_items <= method_call.args.len() + 1 =>
            {
                return Err(FormatErrorKind::Logical.into());
            }

            // We couldn't do horizontal method call arguments, but we can still try vertical, and
            // at this point we don't have any reason not to use it if possible.
            SimulateWrapResult::WrapForLongerFirstLine | SimulateWrapResult::NoWrap => {
                self.out.restore_checkpoint(&checkpoint);
                self.method_call_args_postfix_tail(
                    method_call,
                    ListStrategies::vertical(),
                    postfix_item,
                    tail,
                )?;
            }

            // Use a vertical chain if it has less excess width.
            SimulateWrapResult::WrapForLessExcessWidth => {
                return Err(FormatErrorKind::Logical.into());
            }
        }
        Ok(())
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
                self.method_call(method_call, item, tail)?;
            }
            // root expression
            _ => {
                self.expr_tail(item.root_or_dot_item, Some(&self.tail_fn(postfix_tail)))?;
            }
        }
        Ok(())
    }

    fn method_call(
        &self,
        method_call: &ast::MethodCall,
        postfix_item: &PostfixItem,
        tail: Tail,
    ) -> FormatResult<VSize> {
        self.method_call_dot_path(method_call)?;
        let args_height = self.method_call_args_postfix_tail(
            method_call,
            ListStrategies::flexible_overflow(),
            postfix_item,
            tail,
        )?;
        Ok(args_height)
    }

    fn method_call_dot_path(&self, method_call: &ast::MethodCall) -> FormatResult {
        self.out.token(".")?;
        self.path_segment(&method_call.seg, true, Some(&self.tail_token("(")))?;
        Ok(())
    }

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
                        .next(|| {
                            self.with_single_line(|| {
                                self.expr(index)?;
                                self.out.token("]")?;
                                self.tail(tail)?;
                                Ok(())
                            })
                        })
                        .next(|| {
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
