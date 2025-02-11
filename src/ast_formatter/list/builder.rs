use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::{DefaultListConfig, ListConfig, ListWrapToFitConfig};
use crate::ast_formatter::list::list_item_config::DefaultListItemConfig;
use crate::ast_formatter::list::list_item_context::ListItemContext;
use crate::ast_formatter::list::overflow::{ListOverflow, ListOverflowNo, ListOverflowYes};
use crate::ast_formatter::list::{Braces, ListItemConfig, ListRest};
use crate::ast_formatter::util::tail::Tail;
use crate::constraints::MultiLineConstraint;
use crate::error::FormatResult;

/// Main entrypoint for formatting a list
pub fn list<'ast, 'tail, Item, FormatItem>(
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: FormatItem,
) -> ListBuilder<
    'ast,
    'tail,
    Item,
    FormatItem,
    DefaultListConfig,
    DefaultListItemConfig<Item>,
    ListOverflowNo<Item>,
>
where
    FormatItem: Fn(&AstFormatter, &Item, &ListItemContext) -> FormatResult,
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
    tail: &'tail Tail<'ast>,
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
    FormatItem: Fn(&AstFormatter, &Item, &ListItemContext) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    pub fn config<ConfigNew: ListConfig>(
        self,
        config: ConfigNew,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, ConfigNew, ItemConfig, Overflow> {
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
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig, ListOverflowYes<Item>> {
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
        tail: &'tail_new Tail<'ast>,
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
        af.with_single_line_opt(
            af.constraints().multi_line.get() == MultiLineConstraint::SingleLineLists,
            || self.do_format(af, Self::contents_default),
        )
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
        let mut backtrack = af.backtrack();
        if !(
            ItemConfig::ITEMS_MAY_REQUIRE_OWN_LINE
                && self.list.iter().any(ItemConfig::item_requires_own_line)
        ) {
            backtrack = backtrack.next(|| self.contents_single_line(af));
        }
        match Config::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                assert!(
                    matches!(self.rest, ListRest::None),
                    "rest cannot be used with wrap-to-fit"
                );
                backtrack = backtrack
                    .next(|| self.contents_wrap_to_fit(af, max_element_width));
            }
            ListWrapToFitConfig::No => {}
        }
        backtrack.otherwise(|| self.contents_separate_lines(af))
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
    FormatItem: Fn(&AstFormatter, &Item, &ListItemContext) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    fn format(&self, af: &AstFormatter) -> FormatResult {
        self.format(af)
    }

    fn format_single_line(&self, af: &AstFormatter) -> FormatResult {
        self.format_single_line(af)
    }
}
