use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::fallback_chain::{HasSourceFormatter, fallback_chain};
use crate::ast_formatter::last_line::Tail;
use crate::source_formatter::{FormatResult, SourceFormatter};
use std::marker::PhantomData;

use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_ast::ptr::P;

pub trait ListConfig {
    const START_BRACE: &'static str;
    const END_BRACE: &'static str;
    const PAD_CONTENTS: bool;

    fn single_line_max_contents_width(&self) -> Option<usize> {
        None
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<usize> },
}

pub struct AngleBracketedListConfig;
impl ListConfig for AngleBracketedListConfig {
    const START_BRACE: &'static str = "<";
    const END_BRACE: &'static str = ">";
    const PAD_CONTENTS: bool = false;
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    const START_BRACE: &'static str = "[";
    const END_BRACE: &'static str = "]";
    const PAD_CONTENTS: bool = false;

    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.array_width)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        // short_array_element_width_threshold in rustfmt
        ListWrapToFitConfig::Yes {
            max_element_width: Some(10),
        }
    }
}

pub fn param_list_config(
    single_line_max_contents_width: Option<usize>,
) -> impl ListConfig {
    struct ParamListConfig {
        single_line_max_contents_width: Option<usize>,
    }
    impl ListConfig for ParamListConfig {
        const START_BRACE: &'static str = "(";
        const END_BRACE: &'static str = ")";
        const PAD_CONTENTS: bool = false;

        fn single_line_max_contents_width(&self) -> Option<usize> {
            self.single_line_max_contents_width
        }
    }
    ParamListConfig {
        single_line_max_contents_width,
    }
}

pub struct StructFieldListConfig;

impl ListConfig for StructFieldListConfig {
    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = true;

    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
    }
}

pub trait ListOverflow {
    type Item;
    
    fn format_if_overflow(
        _ast_formatter: &mut AstFormatter<'_>,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> Option<FormatResult> {
        None
    } 
}

pub fn list_overflow_no<T>() -> impl ListOverflow<Item = T> {
    ListOverflowNo(PhantomData)
}

pub fn list_overflow_yes<T: Overflow>() -> impl ListOverflow<Item = T> {
    ListOverflowYes(PhantomData)
}

struct ListOverflowNo<T>(PhantomData<T>);
struct ListOverflowYes<T>(PhantomData<T>);

impl<T> ListOverflow for ListOverflowNo<T> {
    type Item = T;
    
    fn format_if_overflow(
        _ast_formatter: &mut AstFormatter<'_>,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> Option<FormatResult> {
        None
    }
}


impl<T: Overflow> ListOverflow for ListOverflowYes<T> {
    type Item = T;

    fn format_if_overflow(
        ast_formatter: &mut AstFormatter<'_>,
        item: &Self::Item,
        is_only_item: bool,
    ) -> Option<FormatResult> {
        Overflow::format_if_overflow(ast_formatter, item, is_only_item)
    } 
}

impl<'a> AstFormatter<'a> {
    pub fn list<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        config: C,
        overflow: impl ListOverflow<Item=T>,
        tail: Tail,
    ) -> FormatResult {
        self.format_list(
            C::START_BRACE,
            C::END_BRACE,
            list.is_empty(),
            |this, tail| this.list_non_empty_contents_default(list, format_item, overflow, config, tail),
            tail,
        )
    }

    pub fn list_separate_lines<T>(
        &mut self,
        list: &[T],
        start_brace: &'static str,
        end_brace: &'static str,
        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        end: Tail,
    ) -> FormatResult {
        self.format_list(
            start_brace,
            end_brace,
            list.is_empty(),
            |this, tail| this.list_contents_separate_lines(list, format_item, tail),
            end,
        )
    }

    fn format_list<'b, 'c>(
        &mut self,
        start_brace: &'static str,
        end_brace: &'static str,
        is_empty: bool,
        non_empty: impl FnOnce(&mut AstFormatter<'a>, Tail) -> FormatResult + 'b,
        end: Tail,
    ) -> FormatResult {
        self.out.token_expect(start_brace)?;
        if is_empty {
            self.out.token_expect(end_brace)?;
            return self.tail(&end);
        }
        non_empty(
            self,
            Tail::new(move |this| {
                this.out.token_expect(end_brace)?;
                this.tail(&end)?;
                Ok(())
            }),
        )
    }

    fn list_non_empty_contents_default<T, Config>(
        &mut self,
        list: &[T],
        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        overflow: impl ListOverflow<Item=T>,
        config: Config,
        end: Tail,
    ) -> FormatResult
    where
        Config: ListConfig,
    {
        fallback_chain(
            self,
            |chain| {
                chain.next(|this| {
                    this.list_contents_single_line(list, format_item, overflow, Config::PAD_CONTENTS,
                                                   config.single_line_max_contents_width(),
                    )
                });
                match Config::wrap_to_fit() {
                    ListWrapToFitConfig::Yes { max_element_width } => {
                        chain.next(move |this| {
                            this.list_contents_wrap_to_fit(list, format_item, max_element_width)
                        });
                    }
                    ListWrapToFitConfig::No => {}
                }
                chain.next(|this| this.list_contents_separate_lines(list, format_item, Tail::NONE));
            },
            |this| this.tail(&end),
        )
    }

    fn list_contents_single_line<Item, Overflow: ListOverflow<Item=Item>>(
        &mut self,
        list: &[Item],
        format_item: impl (Fn(&mut Self, &Item) -> FormatResult) + Copy,
        _overflow: Overflow,
        pad_contents: bool,
        max_width: Option<usize>,
    ) -> FormatResult
    {
        if pad_contents {
            self.out.space()?;
        }
        let contents = |this: &mut Self| {
            let [items_except_last @ .., last] = list else {
                unreachable!()
            };
            this.with_single_line(|this| {
                for item in items_except_last {
                    format_item(this, item)?;
                    this.out.token_maybe_missing(",")?;
                    this.out.space()?;
                }
                Ok(())
            })?;
            if let Some(result) = Overflow::format_if_overflow(this, last, list.len() == 1) {
                result?;
            } else {
                this.with_single_line(|this| format_item(this, last))?;
            }
            this.out.skip_token_if_present(",");
            Ok(())
        };
        if let Some(max_width) = max_width {
            self.with_width_limit_first_line(max_width, |this| contents(this))?;
        } else {
            contents(self)?;
        }
        if pad_contents {
            self.out.space()?;
        }
        Ok(())
    }

    fn list_contents_wrap_to_fit<T>(
        &mut self,

        list: &[T],
        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        max_element_width: Option<usize>,
    ) -> FormatResult {
        let format_item = |this: &mut Self, item| match max_element_width {
            Some(max_width) => {
                this.with_width_limit_single_line(max_width, |this| format_item(this, item))
            }
            None => format_item(this, item),
        };
        self.indented(|this| {
            this.out.newline_indent()?;
            let [head, tail @ ..] = list else {
                unreachable!()
            };
            format_item(this, head)?;
            this.out.token_maybe_missing(",")?;
            for item in tail {
                this.fallback_chain(
                    |chain| {
                        chain.next(|this| this.out.space());
                        chain.next(|this| this.out.newline_indent());
                    },
                    |this| {
                        format_item(this, item)?;
                        this.out.token_maybe_missing(",")?;
                        Ok(())
                    },
                )?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        Ok(())
    }

    fn list_contents_separate_lines<T>(
        &mut self,
        list: &[T],

        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        tail: Tail,
    ) -> FormatResult {
        self.indented(|this| {
            for item in list {
                this.out.newline_indent()?;
                format_item(this, item)?;
                this.out.token_maybe_missing(",")?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.tail(&tail)?;
        Ok(())
    }
}

trait OverflowHandler {
    type Result;

    const FORMATTING: bool;

    fn no_overflow() -> Self::Result;
    fn overflows(
        this: &mut AstFormatter<'_>,
        format: impl FnOnce(&mut AstFormatter<'_>) -> FormatResult,
    ) -> Self::Result;
}

struct CheckIfOverflow;
struct OverflowDoFormat;

impl OverflowHandler for CheckIfOverflow {
    type Result = bool;

    const FORMATTING: bool = false;

    fn no_overflow() -> bool {
        false
    }

    fn overflows(
        _this: &mut AstFormatter<'_>,
        _format: impl FnOnce(&mut AstFormatter<'_>) -> FormatResult,
    ) -> bool {
        true
    }
}

impl OverflowHandler for OverflowDoFormat {
    type Result = FormatResult;

    const FORMATTING: bool = true;

    fn no_overflow() -> FormatResult {
        unreachable!()
    }

    fn overflows(
        this: &mut AstFormatter<'_>,
        format: impl FnOnce(&mut AstFormatter<'_>) -> FormatResult,
    ) -> FormatResult {
        this.with_not_single_line(format)
    }
}

trait Overflow {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &mut AstFormatter<'_>,
        t: &Self,
        is_only_list_item: bool,
    ) -> H::Result;

    fn check_if_overflows(this: &mut AstFormatter<'_>, t: &Self, is_only_list_item: bool) -> bool {
        Self::format_or_check_if_overflow::<CheckIfOverflow>(this, t, is_only_list_item)
    }

    fn format(this: &mut AstFormatter<'_>, t: &Self, is_only_list_item: bool) -> FormatResult {
        Self::format_or_check_if_overflow::<OverflowDoFormat>(this, t, is_only_list_item)
    }

    fn format_if_overflow(
        this: &mut AstFormatter<'_>,
        t: &Self,
        is_only_list_item: bool,
    ) -> Option<FormatResult> {
        if Self::check_if_overflows(this, t, is_only_list_item) {
            Some(Self::format(this, t, is_only_list_item))
        } else {
            None
        }
    }
}

impl Overflow for ast::Expr {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &mut AstFormatter<'_>,
        expr: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        if !expr.attrs.is_empty() {
            return H::no_overflow();
        }
        match expr.kind {
            // block-like
            ast::ExprKind::Block(..) | ast::ExprKind::Closure(..) | ast::ExprKind::Gen(..) => {
                H::overflows(this, |this| this.expr(expr, Tail::NONE))
            }
            // control flow
            // | ast::ExprKind::ForLoop { .. }
            // | ast::ExprKind::If(..)
            // | ast::ExprKind::Loop(..)
            // | ast::ExprKind::Match(..)
            // | ast::ExprKind::While(..)
            // list
            // | ast::ExprKind::Array(..)
            // | ast::ExprKind::Call(..)
            // | ast::ExprKind::MacCall(..)
            // | ast::ExprKind::Struct(..)
            // | ast::ExprKind::Tup(..) if is_only_list_item => H::overflows(|| this.expr(expr, Tail::None)),
            // | ast::ExprKind::MethodCall(..) if is_only_list_item => H::overflows(|| this.dot_chain(expr, Tail::NONE, true)),
            // prefix
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref inner)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, inner, is_only_list_item) =>
            {
                H::overflows(this, |this| {
                    this.addr_of(borrow_kind, mutability, expr)?;
                    Overflow::format(this, inner, is_only_list_item)
                })
            }
            ast::ExprKind::Cast(ref target, _)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Try(ref target)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Unary(_, ref target)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            _ => H::no_overflow(),
        }
    }
}

impl Overflow for ast::MetaItemInner {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &mut AstFormatter<'_>,
        item: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        match item {
            ast::MetaItemInner::Lit(..) => H::overflows(this, |this| todo!()),
            ast::MetaItemInner::MetaItem(meta_item) => {
                if matches!(meta_item.kind, ast::MetaItemKind::Word) {
                    H::overflows(this, |this| this.meta_item(meta_item))
                } else {
                    H::no_overflow()
                }
            }
        }
    }
}

impl<T: Overflow> Overflow for P<T> {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &mut AstFormatter<'_>,
        t: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        <T as Overflow>::format_or_check_if_overflow::<H>(this, t, is_only_list_item)
    }
}
