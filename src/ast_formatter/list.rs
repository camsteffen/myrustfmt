use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::fallback_chain::{HasSourceFormatter, fallback_chain};
use crate::ast_formatter::last_line::{EndReserved, Tail, drop_end_reserved};
use crate::source_formatter::{FormatResult, SourceFormatter};
use std::marker::PhantomData;

use rustc_ast::ast;
use rustc_ast::ptr::P;

pub trait ListConfig {
    type Item;

    const START_BRACE: &'static str;
    const END_BRACE: &'static str;
    const PAD_CONTENTS: bool;

    fn single_line_max_contents_width() -> Option<usize> {
        None
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }

    fn allow_item_overflow(_item: &Self::Item, _is_only_list_item: bool) -> bool {
        false
    }
}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<usize> },
}

pub fn angle_bracketed_list_config<T>() -> impl ListConfig<Item = T> {
    struct Config<T>(PhantomData<T>);
    impl<T> ListConfig for Config<T> {
        type Item = T;

        const START_BRACE: &'static str = "<";
        const END_BRACE: &'static str = ">";
        const PAD_CONTENTS: bool = false;
    }
    Config(PhantomData::<T>)
}

pub fn array_list_config<T: Overflow>() -> impl ListConfig<Item = T> {
    struct Config<T>(PhantomData<T>);
    impl<T: Overflow> ListConfig for Config<T> {
        type Item = T;

        const START_BRACE: &'static str = "[";
        const END_BRACE: &'static str = "]";
        const PAD_CONTENTS: bool = false;

        fn single_line_max_contents_width() -> Option<usize> {
            // array_width in rustfmt
            Some(60)
        }

        fn wrap_to_fit() -> ListWrapToFitConfig {
            // short_array_element_width_threshold in rustfmt
            ListWrapToFitConfig::Yes {
                max_element_width: Some(10),
            }
        }

        fn allow_item_overflow(item: &Self::Item, is_only_list_item: bool) -> bool {
            <T as Overflow>::allow_overflow(item, is_only_list_item)
        }
    }
    Config(PhantomData::<T>)
}

pub fn param_list_config<T: Overflow>() -> impl ListConfig<Item = T> {
    struct Config<T>(PhantomData<T>);
    impl<T: Overflow> ListConfig for Config<T> {
        type Item = T;

        const START_BRACE: &'static str = "(";
        const END_BRACE: &'static str = ")";
        const PAD_CONTENTS: bool = false;

        fn allow_item_overflow(item: &Self::Item, is_only_list_item: bool) -> bool {
            <T as Overflow>::allow_overflow(item, is_only_list_item)
        }
    }
    Config(PhantomData::<T>)
}

pub fn param_list_no_overflow_config<T>() -> impl ListConfig<Item = T> {
    struct Config<T>(PhantomData<T>);
    impl<T> ListConfig for Config<T> {
        type Item = T;

        const START_BRACE: &'static str = "(";
        const END_BRACE: &'static str = ")";
        const PAD_CONTENTS: bool = false;
    }
    Config(PhantomData::<T>)
}

pub struct PatFieldListConfig;

impl ListConfig for PatFieldListConfig {
    type Item = ast::PatField;

    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = true;

    fn single_line_max_contents_width() -> Option<usize> {
        // struct_lit_width in rustfmt
        Some(18)
    }
}

impl<'a> AstFormatter<'a> {
    pub fn list<T, C: ListConfig<Item = T>>(
        &mut self,
        list: &[T],
        format_item: impl (Fn(&mut Self, &T) -> FormatResult) + Copy,
        config: C,
        end: Tail,
    ) -> FormatResult {
        ListContext {
            ast_formatter: self,
            list,
            format_item,
            end,
            config,
        }
        .format()
    }
}

struct ListContext<'source, 'list, 'a, Item, FormatItem, Config> {
    ast_formatter: &'a mut AstFormatter<'source>,
    list: &'list [Item],
    format_item: FormatItem,
    end: Tail,
    config: Config,
}

impl<'source, 'list, 'a, Item, FormatItem, Config> HasSourceFormatter<'source>
    for ListContext<'source, '_, '_, Item, FormatItem, Config>
{
    fn source_formatter(&mut self) -> &mut SourceFormatter<'source> {
        &mut self.ast_formatter.out
    }
}

impl<'source: 'list, 'list, 'a, Item, FormatItem, Config>
    ListContext<'source, 'list, '_, Item, FormatItem, Config>
where
    FormatItem: (Fn(&mut AstFormatter<'source>, &Item) -> FormatResult) + Copy,
    Config: ListConfig<Item = Item>,
{
    fn format(&mut self) -> FormatResult {
        let ListContext {
            ast_formatter: ref mut this,
            end,
            ..
        } = *self;
        this.out.token_expect(Config::START_BRACE)?;
        if self.list.is_empty() {
            this.out.token_expect(Config::END_BRACE)?;
            return this.tail(end);
        }
        fallback_chain(
            self,
            |chain| {
                chain.next(|this| this.list_single_line());
                match Config::wrap_to_fit() {
                    ListWrapToFitConfig::Yes { max_element_width } => {
                        chain.next(move |this| this.list_wrap_to_fit(max_element_width));
                    }
                    ListWrapToFitConfig::No => {}
                }
                chain.next(|this| this.list_separate_lines());
            },
            |this| {
                this.ast_formatter.out.token_expect(Config::END_BRACE)?;
                this.ast_formatter.tail(end)
            },
        )
    }

    fn list_single_line(&mut self) -> FormatResult {
        let ListContext {
            ast_formatter: this,
            list,
            format_item,
            config,
            end,
        } = self;
        if Config::PAD_CONTENTS {
            this.out.space()?;
        }
        let contents = |this: &mut AstFormatter<'source>| {
            this.with_single_line(|this| {
                for item in &list[..list.len() - 1] {
                    format_item(this, item)?;
                    this.out.token_maybe_missing(",")?;
                    this.out.space()?;
                }
                Ok(())
            })?;
            let last = list.last().expect("list shouldn't be empty");
            if Config::allow_item_overflow(last, list.len() == 1) {
                format_item(this, last)?;
            } else {
                this.with_single_line(|this| format_item(this, last))?;
            }
            this.out.skip_token_if_present(",");
            Ok(())
        };
        if let Some(max_width) = Config::single_line_max_contents_width() {
            this.with_width_limit_first_line(max_width, |this| contents(this))?;
        } else {
            contents(this)?;
        }
        if Config::PAD_CONTENTS {
            this.out.space()?;
        }
        Ok(())
    }

    fn list_wrap_to_fit(&mut self, max_element_width: Option<usize>) -> FormatResult {
        let ListContext {
            ast_formatter: this,
            list,
            format_item,
            config,
            end,
        } = self;
        let format_item = |this: &mut AstFormatter<'source>, item| match max_element_width {
            Some(max_width) => {
                this.with_width_limit_single_line(max_width, |this| format_item(this, item))
            }
            None => format_item(this, item),
        };
        this.indented(|this| {
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
        this.out.newline_indent()?;
        Ok(())
    }

    fn list_separate_lines(&mut self) -> FormatResult {
        let ListContext {
            ast_formatter: ref mut this,
            list,
            format_item,
            ..
        } = *self;
        this.indented(|this| {
            for item in list {
                this.out.newline_indent()?;
                format_item(this, item)?;
                this.out.token_maybe_missing(",")?;
            }
            Ok(())
        })?;
        this.out.newline_indent()?;
        Ok(())
    }
}

trait Overflow {
    fn allow_overflow(&self, is_only_list_item: bool) -> bool;
}

impl Overflow for ast::Expr {
    fn allow_overflow(&self, is_only_list_item: bool) -> bool {
        if !self.attrs.is_empty() {
            return false;
        }
        match &self.kind {
            // block-like
            | ast::ExprKind::Block(..)
            | ast::ExprKind::Closure(..)
            | ast::ExprKind::Gen(..) => true,
            // control flow
            | ast::ExprKind::ForLoop { .. }
            | ast::ExprKind::If(..)
            | ast::ExprKind::Loop(..)
            | ast::ExprKind::Match(..)
            | ast::ExprKind::While(..)
            // list
            | ast::ExprKind::Array(..)
            | ast::ExprKind::Call(..)
            | ast::ExprKind::MacCall(..)
            | ast::ExprKind::MethodCall(..)
            | ast::ExprKind::Struct(..)
            | ast::ExprKind::Tup(..) => is_only_list_item,
            // prefix
            | ast::ExprKind::AddrOf(_, _, expr)
            | ast::ExprKind::Cast(expr, _)
            | ast::ExprKind::Try(expr)
            | ast::ExprKind::Unary(_, expr) => expr.allow_overflow(is_only_list_item),
            _ => false,
        }
    }
}

impl Overflow for ast::MetaItemInner {
    fn allow_overflow(&self, _is_only_list_item: bool) -> bool {
        match self {
            ast::MetaItemInner::Lit(..) => true,
            ast::MetaItemInner::MetaItem(meta_item) => {
                matches!(meta_item.kind, ast::MetaItemKind::Word)
            }
        }
    }
}

impl<T: Overflow> Overflow for P<T> {
    fn allow_overflow(&self, is_only_list_item: bool) -> bool {
        <T as Overflow>::allow_overflow(self, is_only_list_item)
    }
}
