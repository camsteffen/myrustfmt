use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::marker::PhantomData;

pub trait ListOverflow: Copy {
    type Item;

    fn can_overflow(
        _ast_formatter: &AstFormatter,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> bool {
        false
    }

    fn format_overflow(
        _ast_formatter: &AstFormatter,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> FormatResult {
        panic!("not supported")
    }
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
}

impl<T: Overflow> ListOverflow for ListOverflowYes<T> {
    type Item = T;

    fn can_overflow(ast_formatter: &AstFormatter, item: &Self::Item, is_only_item: bool) -> bool {
        Overflow::check_if_overflows(ast_formatter, item, is_only_item)
    }

    fn format_overflow(
        ast_formatter: &AstFormatter,
        item: &Self::Item,
        is_only_item: bool,
    ) -> FormatResult {
        Overflow::format(ast_formatter, item, is_only_item)
    }
}

trait OverflowHandler {
    type Result;

    const FORMATTING: bool;

    fn no_overflow() -> Self::Result;
    fn overflows(format: impl Fn() -> FormatResult) -> Self::Result;
}

struct CheckIfOverflow;

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
}

trait Overflow {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        t: &Self,
        is_only_list_item: bool,
    ) -> H::Result;

    fn check_if_overflows(this: &AstFormatter, t: &Self, is_only_list_item: bool) -> bool {
        Self::format_or_check_if_overflow::<CheckIfOverflow>(this, t, is_only_list_item)
    }

    fn format(this: &AstFormatter, t: &Self, is_only_list_item: bool) -> FormatResult {
        Self::format_or_check_if_overflow::<OverflowDoFormat>(this, t, is_only_list_item)
    }
}

impl Overflow for ast::Expr {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        af: &AstFormatter,
        expr: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        if !expr.attrs.is_empty() {
            return H::no_overflow();
        }
        match expr.kind {
            // block-like
            ast::ExprKind::Block(..) | ast::ExprKind::Gen(..) => H::overflows(|| af.expr(expr)),
            ast::ExprKind::Closure(ref closure) => {
                H::overflows(|| af.closure(closure, true, Tail::NONE))
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
            | ast::ExprKind::Tup(..)
                if is_only_list_item =>
            {
                H::overflows(|| af.with_do_overflow(|| af.expr(expr)))
            }
            // | ast::ExprKind::MethodCall(..) if is_only_list_item => H::overflows(|| af.dot_chain(expr, Tail::NONE, true)),
            // prefix
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target)
                if H::FORMATTING || Overflow::check_if_overflows(af, target, is_only_list_item) =>
            {
                H::overflows(|| {
                    af.addr_of(borrow_kind, mutability, expr)?;
                    Overflow::format(af, target, is_only_list_item)
                })
            }
            ast::ExprKind::Cast(ref target, _)
                if H::FORMATTING || Overflow::check_if_overflows(af, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Try(ref target)
                if H::FORMATTING || Overflow::check_if_overflows(af, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Unary(_, ref target)
                if H::FORMATTING || Overflow::check_if_overflows(af, target, is_only_list_item) =>
            {
                todo!()
            }
            _ => H::no_overflow(),
        }
    }
}

impl Overflow for ast::MetaItemInner {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        af: &AstFormatter,
        item: &Self,
        _is_only_list_item: bool,
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
        is_only_list_item: bool,
    ) -> H::Result {
        <T as Overflow>::format_or_check_if_overflow::<H>(this, t, is_only_list_item)
    }
}
