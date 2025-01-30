mod braces;
pub mod list_config;
mod list_item_config;
mod overflow;
mod rest;

pub use braces::Braces;
pub use list_item_config::ListItemConfig;
pub use rest::ListRest;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::{DefaultListConfig, ListConfig, ListWrapToFitConfig};
use crate::ast_formatter::list::list_item_config::DefaultListItemConfig;
use crate::ast_formatter::util::tail::Tail;
use crate::error::{ConstraintError, FormatResult};
use overflow::{ListOverflow, ListOverflowNo, ListOverflowYes};

/// Main entrypoint for formatting a list
pub fn list<'a, 'list, Item, FormatItem>(
    braces: &'static Braces,
    list: &'list [Item],
    format_item: FormatItem,
) -> ListBuilder<
    'list,
    'static,
    Item,
    FormatItem,
    DefaultListConfig,
    DefaultListItemConfig<Item>,
    ListOverflowNo<Item>,
>
where
    FormatItem: Fn(&Item) -> FormatResult,
{
    ListBuilder {
        braces,
        list,
        rest: ListRest::None,
        format_item,
        tail: &Tail::none(),
        config: DefaultListConfig,
        item_config: DefaultListItemConfig::default(),
        overflow: ListOverflowNo::default(),
        omit_open_brace: false,
    }
}

// Yikes, lots of generics here. This allows the compiler to optimize away unneeded features.
// The monomorphization shouldn't be too much since there is a finite number of list cases, and the
// builder delegates to less generic functions for the actual formatting implementation.
pub struct ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig, Overflow> {
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: FormatItem,
    rest: ListRest<'ast>,
    tail: &'tail Tail,
    config: Config,
    item_config: ItemConfig,
    overflow: Overflow,
    omit_open_brace: bool,
}

impl<'a, 'ast, 'tail, Item, FormatItem, Config, ItemConfig, Overflow>
    ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig, Overflow>
where
    Config: ListConfig,
    ItemConfig: ListItemConfig<Item = Item>,
    FormatItem: Fn(&Item) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    pub fn config<ConfigNew: ListConfig>(
        self,
        config: ConfigNew,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, ConfigNew, ItemConfig, Overflow>
    {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config,
            item_config: self.item_config,
            overflow: self.overflow,
            omit_open_brace: self.omit_open_brace,
        }
    }

    pub fn item_config<ItemConfigNew: ListItemConfig<Item = Item>>(
        self,
        item_config: ItemConfigNew,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfigNew, Overflow> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config: self.config,
            item_config,
            overflow: self.overflow,
            omit_open_brace: self.omit_open_brace,
        }
    }

    pub fn overflow(
        self,
    ) -> ListBuilder<
        'ast,
        'tail,
        Item,
        FormatItem,
        Config,
        ItemConfig,
        ListOverflowYes<Item>,
    > {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config: self.config,
            item_config: self.item_config,
            overflow: ListOverflowYes::default(),
            omit_open_brace: self.omit_open_brace,
        }
    }

    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListBuilder { rest, ..self }
    }

    pub fn tail<'tail_new>(
        self,
        tail: &'tail_new Tail,
    ) -> ListBuilder<'ast, 'tail_new, Item, FormatItem, Config, ItemConfig, Overflow> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail,
            config: self.config,
            item_config: self.item_config,
            overflow: self.overflow,
            omit_open_brace: self.omit_open_brace,
        }
    }

    pub fn omit_open_brace(self) -> Self {
        ListBuilder {
            omit_open_brace: true,
            ..self
        }
    }

    pub fn format(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_default)
    }

    pub fn format_single_line(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_single_line)
    }

    pub fn format_separate_lines(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_separate_lines)
    }

    fn do_format(
        &self,
        af: &AstFormatter,
        contents: impl FnOnce(&Self, &AstFormatter) -> FormatResult,
    ) -> FormatResult {
        if !self.omit_open_brace {
            af.out.token(self.braces.start)?;
        }
        if self.list.is_empty() && matches!(self.rest, ListRest::None) {
            af.embraced_empty_after_opening(self.braces.end)?;
            af.tail(self.tail)?;
            return Ok(());
        }
        contents(self, af)
    }

    fn contents_default(&self, af: &AstFormatter) -> FormatResult {
        let mut fallback = af.fallback(|| {
            if ItemConfig::ITEMS_POSSIBLY_MUST_HAVE_OWN_LINE
                && self.list.iter().any(ItemConfig::item_must_have_own_line)
            {
                return Err(ConstraintError::Logical.into());
            }
            self.contents_single_line(af)
        });
        match Config::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                assert!(
                    matches!(self.rest, ListRest::None),
                    "rest cannot be used with wrap-to-fit"
                );
                fallback = fallback.next(|| self.contents_wrap_to_fit(af, max_element_width));
            }
            ListWrapToFitConfig::No => {}
        }
        fallback.otherwise(|| self.contents_separate_lines(af))
    }

    fn contents_single_line(&self, af: &AstFormatter) -> FormatResult {
        af.list_contents_single_line(
            self.list,
            &self.format_item,
            self.rest,
            self.braces.end,
            self.tail,
            self.overflow,
            self.config.force_trailing_comma(),
            self.braces.pad,
            self.config.single_line_max_contents_width(),
            self.config
                .overflow_max_first_line_contents_width(af.config()),
        )
    }

    fn contents_wrap_to_fit(
        &self,
        af: &AstFormatter,
        max_element_width: Option<u32>,
    ) -> FormatResult {
        af.list_contents_wrap_to_fit(
            self.list,
            self.braces.end,
            self.tail,
            &self.format_item,
            self.item_config,
            max_element_width,
        )
    }

    fn contents_separate_lines(&self, af: &AstFormatter) -> FormatResult {
        af.list_contents_separate_lines(
            self.list,
            &self.format_item,
            self.rest,
            self.braces.end,
            self.tail,
        )
    }
}

/// convenience for `-> impl ListBuilderTrait`, otherwise ListBuilder is preferred
pub trait ListBuilderTrait {
    fn format(&self, af: &AstFormatter) -> FormatResult;

    fn format_single_line(&self, af: &AstFormatter) -> FormatResult;
}

impl<'a, 'ast, 'tail, Item, FormatItem, Config, ItemConfig, Overflow> ListBuilderTrait
    for ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig, Overflow>
where
    Config: ListConfig,
    ItemConfig: ListItemConfig<Item = Item>,
    FormatItem: Fn(&Item) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    fn format(&self, af: &AstFormatter) -> FormatResult {
        self.format(af)
    }

    fn format_single_line(&self, af: &AstFormatter) -> FormatResult {
        self.format_single_line(af)
    }
}

impl AstFormatter {
    /* [item, item, item] */
    fn list_contents_single_line<Item, Overflow: ListOverflow<Item = Item>>(
        &self,
        list: &[Item],
        format_item: impl Fn(&Item) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        tail: &Tail,
        _overflow: Overflow,
        force_trailing_comma: bool,
        pad: bool,
        max_width: Option<u32>,
        max_width_overflow: Option<u32>,
    ) -> FormatResult {
        if pad {
            self.out.space()?;
        }
        self.with_width_limit_first_line_opt(max_width, || -> FormatResult {
            let Some((last, until_last)) = list.split_last() else {
                if !matches!(rest, ListRest::None) {
                    self.list_rest(rest)?;
                }
                return Ok(());
            };
            let start = self.out.last_line_len();
            self.with_single_line(|| -> FormatResult {
                for item in until_last {
                    format_item(item)?;
                    self.out.token_maybe_missing(",")?;
                    self.out.space()?;
                }
                Ok(())
            })?;
            let trailing_comma = || {
                if !matches!(rest, ListRest::None) || force_trailing_comma {
                    self.out.token(",")
                } else {
                    self.out.skip_token_if_present(",")
                }
            };
            let last_without_overflow = || {
                self.with_single_line(|| {
                    format_item(last)?;
                    trailing_comma()?;
                    if !matches!(rest, ListRest::None) {
                        self.out.space()?;
                        self.list_rest(rest)?;
                    }
                    Ok(())
                })
            };
            let can_overflow = matches!(rest, ListRest::None)
                && Overflow::can_overflow(self, last, list.len() == 1);
            if can_overflow {
                self.fallback(last_without_overflow).otherwise(|| {
                    self.with_width_limit_from_start_first_line_opt(
                        start,
                        max_width_overflow,
                        || {
                            Overflow::format_overflow(self, last)?;
                            trailing_comma()?;
                            Ok(())
                        },
                    )
                })?;
            } else {
                last_without_overflow()?;
            }
            Ok(())
        })?;
        if pad {
            self.out.space()?;
        }
        self.out.token(close_brace)?;
        self.tail(tail)?;
        Ok(())
    }

    /*
    [
        item, item, item,
        item,
    ]
    */
    // todo how does this behave with comments between items - forcing newlines?
    fn list_contents_wrap_to_fit<T, ItemConfig>(
        &self,
        list: &[T],
        close_brace: &str,
        tail: &Tail,
        format_item: impl Fn(&T) -> FormatResult,
        _item_config: ItemConfig,
        max_element_width: Option<u32>,
    ) -> FormatResult
    where
        ItemConfig: ListItemConfig<Item = T>,
    {
        let format_item = |item| match max_element_width {
            Some(max_width) => self.with_single_line_and_width_limit(max_width, || format_item(item)),
            None => format_item(item),
        };
        self.embraced_after_opening(close_brace, || {
            let (first, rest) = list.split_first().unwrap();
            format_item(first)?;
            self.out.token_maybe_missing(",")?;
            let mut prev_must_have_own_line = false;
            for item in rest {
                let item_comma = || -> FormatResult {
                    format_item(item)?;
                    self.out.token_maybe_missing(",")?;
                    Ok(())
                };
                let item_same_line = || {
                    self.out.space()?;
                    item_comma()?;
                    Ok(())
                };
                let item_next_line = || {
                    self.out.newline_within_indent()?;
                    item_comma()?;
                    Ok(())
                };
                if prev_must_have_own_line || ItemConfig::item_must_have_own_line(item) {
                    item_next_line()?;
                    prev_must_have_own_line = !prev_must_have_own_line;
                } else {
                    self.fallback(item_same_line).otherwise(item_next_line)?;
                }
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    /*
    [
        item,
        item,
        item,
    ]
    */
    fn list_contents_separate_lines<T>(
        &self,
        list: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        tail: &Tail,
    ) -> FormatResult {
        let item_comma = |item| -> FormatResult {
            format_item(item)?;
            self.out.token_maybe_missing(",")?;
            Ok(())
        };
        self.embraced_after_opening(close_brace, || {
            match rest {
                ListRest::None => {
                    let (last, until_last) = list.split_last().unwrap();
                    for item in until_last {
                        item_comma(item)?;
                        // todo should this be "between"?
                        self.out.newline_within_indent()?;
                    }
                    item_comma(last)?;
                }
                _ => {
                    for item in list {
                        item_comma(item)?;
                        self.out.newline_within_indent()?;
                    }
                    self.list_rest(rest)?;
                }
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }
    
    fn list_rest(&self, rest: ListRest<'_>) -> FormatResult {
        self.out.token("..")?;
        if let ListRest::Base(expr) = rest {
            self.expr(expr)?;
        }
        Ok(())
    }
}
