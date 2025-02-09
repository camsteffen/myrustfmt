use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::marker::PhantomData;

pub trait ListOverflow: Copy {
    type Item;

    fn can_overflow(ast_formatter: &AstFormatter, item: &Self::Item, is_only_item: bool) -> bool;

    fn format_overflow(ast_formatter: &AstFormatter, item: &Self::Item) -> FormatResult;
}

pub struct ListOverflowNo<T>(PhantomData<T>);
pub struct ListOverflowYes<T>(PhantomData<T>);

impl<T> Clone for ListOverflowNo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ListOverflowNo<T> {}
impl<T> Default for ListOverflowNo<T> {
    fn default() -> Self {
        ListOverflowNo(PhantomData)
    }
}

impl<T> Clone for ListOverflowYes<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ListOverflowYes<T> {}
impl<T> Default for ListOverflowYes<T> {
    fn default() -> Self {
        ListOverflowYes(PhantomData)
    }
}

impl<T> ListOverflow for ListOverflowNo<T> {
    type Item = T;

    fn can_overflow(
        _ast_formatter: &AstFormatter,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> bool {
        false
    }

    fn format_overflow(_ast_formatter: &AstFormatter, _item: &Self::Item) -> FormatResult {
        panic!("overflow disabled")
    }
}

impl<T: Overflow> ListOverflow for ListOverflowYes<T> {
    type Item = T;

    fn can_overflow(ast_formatter: &AstFormatter, item: &Self::Item, is_only_item: bool) -> bool {
        Overflow::check_if_overflows(ast_formatter, item, is_only_item)
    }

    fn format_overflow(ast_formatter: &AstFormatter, item: &Self::Item) -> FormatResult {
        Overflow::format(ast_formatter, item)
    }
}

trait OverflowHandler {
    type Result;

    const FORMATTING: bool;

    fn no_overflow() -> Self::Result;
    fn overflows(format: impl Fn() -> FormatResult) -> Self::Result;
    fn conditional_overflows(
        self,
        predicate: impl Fn(/*is_only_list_item*/ bool) -> bool,
        format: impl Fn() -> FormatResult,
    ) -> Self::Result;
}

struct CheckIfOverflow {
    is_only_list_item: bool,
}

struct OverflowDoFormat;

impl OverflowHandler for CheckIfOverflow {
    type Result = bool;

    const FORMATTING: bool = false;

    fn no_overflow() -> bool {
        false
    }

    fn overflows(_format: impl FnOnce() -> FormatResult) -> bool {
        true
    }

    fn conditional_overflows(
        self,
        predicate: impl Fn(/*is_only_list_item*/ bool) -> bool,
        _format: impl Fn() -> FormatResult,
    ) -> bool {
        predicate(self.is_only_list_item)
    }
}

impl OverflowHandler for OverflowDoFormat {
    type Result = FormatResult;

    const FORMATTING: bool = true;

    fn no_overflow() -> FormatResult {
        unreachable!()
    }

    fn overflows(format: impl Fn() -> FormatResult) -> FormatResult {
        format()
    }

    fn conditional_overflows(
        self,
        _predicate: impl Fn(/*is_only_list_item*/ bool) -> bool,
        format: impl Fn() -> FormatResult,
    ) -> FormatResult {
        format()
    }
}

trait Overflow {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        node: &Self,
        handler: H,
    ) -> H::Result;

    fn check_if_overflows(this: &AstFormatter, t: &Self, is_only_list_item: bool) -> bool {
        Self::format_or_check_if_overflow(this, t, CheckIfOverflow { is_only_list_item })
    }

    fn format(this: &AstFormatter, t: &Self) -> FormatResult {
        Self::format_or_check_if_overflow(this, t, OverflowDoFormat)
    }
}

impl Overflow for ast::Expr {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        af: &AstFormatter,
        expr: &Self,
        handler: H,
    ) -> H::Result {
        /*
        non-middle-indented expressions
        zero-indent dot chains
        hanging indent chains
        closures: single-line chains only, middle indented only
        match arm: single-line chains only, middle indented only
        vertical list item: hanging indent chains allowed, middle indented only
        horizontal list item: single-line chains only, middle indent N/A
        
        hanging indent within a list item is okay because
         */
        if !expr.attrs.is_empty() {
            return H::no_overflow();
        }
        match expr.kind {
            // block-like
            ast::ExprKind::Block(..) | ast::ExprKind::Gen(..) => H::overflows(|| af.expr(expr)),
            ast::ExprKind::Closure(ref closure) => {
                H::overflows(|| af.closure(closure, Tail::none()))
            }
            // control flow
            // | ast::ExprKind::ForLoop { .. }
            // | ast::ExprKind::If(..)
            // | ast::ExprKind::Loop(..)
            // | ast::ExprKind::Match(..)
            // | ast::ExprKind::While(..)
            // list
            ast::ExprKind::Array(..)
            | ast::ExprKind::Call(..)
            | ast::ExprKind::MacCall(..)
            | ast::ExprKind::Struct(..)
            | ast::ExprKind::Tup(..) => handler
                .conditional_overflows(|is_only_list_item| is_only_list_item, || af.expr(expr)),
            // | ast::ExprKind::MethodCall(..) if is_only_list_item => H::overflows(|| af.dot_chain(expr, Tail::NONE, true)),
            // prefix
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => handler
                .conditional_overflows(
                    |is_only_list_item| Overflow::check_if_overflows(af, target, is_only_list_item),
                    || {
                        af.addr_of(borrow_kind, mutability)?;
                        Overflow::format(af, target)
                    },
                ),
            ast::ExprKind::Cast(ref target, _) => handler.conditional_overflows(
                |is_only_list_item| Overflow::check_if_overflows(af, target, is_only_list_item),
                || todo!(),
            ),
            ast::ExprKind::Try(ref target) => handler.conditional_overflows(
                |is_only_list_item| Overflow::check_if_overflows(af, target, is_only_list_item),
                || todo!(),
            ),
            ast::ExprKind::Unary(_, ref target) => handler.conditional_overflows(
                |is_only_list_item| Overflow::check_if_overflows(af, target, is_only_list_item),
                || todo!(),
            ),
            _ => H::no_overflow(),
        }
    }
}

impl Overflow for ast::MetaItemInner {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        af: &AstFormatter,
        item: &Self,
        _handler: H,
    ) -> H::Result {
        match item {
            ast::MetaItemInner::Lit(..) => H::overflows(|| todo!()),
            ast::MetaItemInner::MetaItem(meta_item) => {
                if matches!(meta_item.kind, ast::MetaItemKind::Word) {
                    H::overflows(|| af.meta_item(meta_item))
                } else {
                    H::no_overflow()
                }
            }
        }
    }
}

impl<T: Overflow> Overflow for P<T> {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        t: &Self,
        handler: H,
    ) -> H::Result {
        <T as Overflow>::format_or_check_if_overflow(this, t, handler)
    }
}
