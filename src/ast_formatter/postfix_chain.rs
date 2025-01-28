use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::list::ListBuilderTrait;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::expr_kind;
use crate::error::FormatResult;
use crate::error::{FormatError, WidthLimitExceededError};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_span::symbol::Ident;

#[derive(Debug)]
enum PostfixChainItem<'a> {
    Await,
    Field(Ident),
    Index(&'a ast::Expr),
    MethodCall(&'a ast::MethodCall),
    Try,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let (root, chain) = build_postfix_chain(expr);

        let first_line = self.out.line();
        let start_pos = self.out.last_line_len();
        let indent_margin = self.out.constraints().indent.get() + INDENT_WIDTH;

        // touchy margins - everything must be a single line (with overflow)
        if self.out.constraints().touchy_margin.get() {
            self.with_single_line(|| self.expr(root))?;
            self.postfix_chain_single_line(&chain, start_pos, tail)?;
            return Ok(());
        }

        self.expr(root)?;
        let mut chain_rest = chain.as_slice();
        self.postfix_chain_items(take_unbreakables(&mut chain_rest))?;

        // items that start within the first indent-width
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

    fn postfix_chain_separate_lines(
        &self,
        mut chain: &[PostfixChainItem<'_>],
        tail: &Tail,
    ) -> FormatResult {
        while let Some(next) = take_next_with_unbreakables(&mut chain) {
            self.out.newline_within_indent()?;
            self.postfix_chain_items(next)?;
        }
        self.tail(tail)?;
        Ok(())
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
            PostfixChainItem::MethodCall(method_call) => {
                self.out.token(".")?;
                self.path_segment(&method_call.seg, true, &Tail::token("("))?;
                self.call_args_after_open_paren(&method_call.args, Tail::none())
                    .format(self)?;
            }
            PostfixChainItem::Index(index) => {
                self.out.token("[")?;
                self.fallback(|| {
                    self.with_single_line(|| self.expr_tail(index, &Tail::token("]")))
                })
                .otherwise(|| self.embraced_after_opening("]", || self.expr(index)))?;
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
            _ => {
                debug_assert_eq!(matches!(expr.kind, expr_kind::postfix!()), false);
                break;
            }
        };
    }
    items.reverse();
    (expr, items)
}

/// Splits of the trailing method call, with optional unbreakable items after that,
/// or returns None if the chain doesn't end with a method call.
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

fn take_next_with_unbreakables<'a, 'b>(slice: &mut &'a [PostfixChainItem<'b>]) -> Option<&'a [PostfixChainItem<'b>]> { 
    match slice {
        [] => None,
        [_, rest @ ..] => {
            let pos = rest.iter().position(|item| !is_unbreakable(item)).unwrap_or(rest.len());
            let out;
            (out, *slice) = slice.split_at(pos + 1);
            Some(out)
        }
    }
}


fn take_unbreakables<'a, 'b>(slice: &mut &'a [PostfixChainItem<'b>]) -> &'a [PostfixChainItem<'b>] {
    let pos = slice.iter().position(|item| !is_unbreakable(item)).unwrap_or(slice.len());
    let out;
    (out, *slice) = slice.split_at(pos);
    out
}