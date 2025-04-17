use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::{DefaultListConfig, ListConfig, ListWrapToFitConfig};
use crate::ast_formatter::list::list_item_config::DefaultListItemConfig;
use crate::ast_formatter::list::list_item_context::ListItemContext;
use crate::ast_formatter::list::{Braces, ListItemConfig, ListRest, ListStrategy};
use crate::ast_formatter::tail::Tail;
use crate::constraints::VerticalShape;
use crate::error::FormatResult;
use crate::num::HPos;

pub trait FormatListItem<Item> {
    fn format(
        &self,
        af: &AstFormatter,
        item: &Item,
        tail: &Tail,
        lcx: ListItemContext,
    ) -> FormatResult;
}

fn format_list_item_from_fn<Item>(
    format: impl Fn(&AstFormatter, &Item, &Tail, ListItemContext) -> FormatResult,
) -> impl FormatListItem<Item> {
    struct Impl<F>(F);
    impl<F, Item> FormatListItem<Item> for Impl<F>
    where
        F: Fn(&AstFormatter, &Item, &Tail, ListItemContext) -> FormatResult,
    {
        fn format(
            &self,
            af: &AstFormatter,
            item: &Item,
            tail: &Tail,
            lcx: ListItemContext,
        ) -> FormatResult {
            self.0(af, item, tail, lcx)
        }
    }
    Impl(format)
}

/// Main entrypoint for formatting a list
pub fn list<'ast, 'tail, Item>(
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: impl Fn(&AstFormatter, &Item, &Tail, ListItemContext) -> FormatResult,
) -> ListBuilder<
    'ast,
    'tail,
    Item,
    impl FormatListItem<Item>,
    DefaultListConfig,
    DefaultListItemConfig<Item>,
> {
    ListBuilder {
        braces,
        list,
        rest: ListRest::None,
        format_item: format_list_item_from_fn(format_item),
        tail: Tail::none(),
        config: DefaultListConfig,
        item_config: DefaultListItemConfig::default(),
        omit_open_brace: false,
        single_line_max_contents_width: None,
    }
}

// Yikes, lots of generics here. This allows the compiler to optimize away unneeded features.
// The monomorphization shouldn't be too much since there is a finite number of list cases, and the
// builder delegates to less generic functions for the actual formatting implementation.
pub struct ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig> {
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: FormatItem,
    rest: ListRest<'ast>,
    tail: &'tail Tail<'ast>,
    config: Config,
    item_config: ItemConfig,
    omit_open_brace: bool,
    single_line_max_contents_width: Option<HPos>,
}

impl<'ast, 'tail, Item, FormatItem, Config, ItemConfig>
    ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfig>
where
    Config: ListConfig,
    ItemConfig: ListItemConfig<Item = Item>,
    FormatItem: FormatListItem<Item>,
{
    pub fn config<ConfigNew: ListConfig>(
        self,
        config: ConfigNew,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, ConfigNew, ItemConfig> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config,
            item_config: self.item_config,
            omit_open_brace: self.omit_open_brace,
            single_line_max_contents_width: self.single_line_max_contents_width,
        }
    }

    pub fn item_config<ItemConfigNew: ListItemConfig<Item = Item>>(
        self,
        item_config: ItemConfigNew,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, Config, ItemConfigNew> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config: self.config,
            item_config,
            omit_open_brace: self.omit_open_brace,
            single_line_max_contents_width: self.single_line_max_contents_width,
        }
    }

    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListBuilder { rest, ..self }
    }

    pub fn tail<'tail_new>(
        self,
        tail: &'tail_new Tail<'ast>,
    ) -> ListBuilder<'ast, 'tail_new, Item, FormatItem, Config, ItemConfig> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail,
            config: self.config,
            item_config: self.item_config,
            omit_open_brace: self.omit_open_brace,
            single_line_max_contents_width: self.single_line_max_contents_width,
        }
    }

    pub fn omit_open_brace(self) -> Self {
        ListBuilder {
            omit_open_brace: true,
            ..self
        }
    }

    pub fn single_line_max_contents_width(self, width: HPos) -> Self {
        ListBuilder {
            single_line_max_contents_width: Some(width),
            ..self
        }
    }

    pub fn format(&self, af: &AstFormatter) -> FormatResult {
        af.has_vertical_shape(VerticalShape::List, || {
            self.do_format(af, Self::contents_default)
        })
    }

    pub fn format_single_line(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_horizontal)?;
        af.tail(self.tail)?;
        Ok(())
    }

    pub fn format_vertical(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_vertical)
    }

    fn do_format(
        &self,
        af: &AstFormatter,
        contents: impl FnOnce(&Self, &AstFormatter) -> FormatResult,
    ) -> FormatResult {
        if !self.omit_open_brace {
            af.out.token(self.braces.start)?;
        }
        if self.list.is_empty() && self.rest.is_none() {
            af.enclosed_empty_after_opening(self.braces.end)?;
            af.tail(self.tail)?;
            return Ok(());
        }
        contents(self, af)
    }

    // todo better name
    fn contents_default(&self, af: &AstFormatter) -> FormatResult {
        let first_line = af.out.line();
        let checkpoint = af.out.checkpoint();
        #[derive(Debug)]
        enum HorizontalResult {
            Skip,
            Fail,
            Ok { height: u32 },
        }
        let result = af.out.with_enforce_max_width(|| -> FormatResult<_> {
            if self.list.iter().any(ItemConfig::item_requires_own_line) {
                return Ok(HorizontalResult::Skip);
            }
            let result1 = self.contents_horizontal(af);
            if result1.is_err() {
                return Ok(HorizontalResult::Fail);
            }
            // N.B. measure before writing the tail
            let height = af.out.line() - first_line + 1;
            if af.tail(self.tail).is_err() {
                return Ok(HorizontalResult::Fail);
            }
            Ok(HorizontalResult::Ok { height })
        })?;

        match result {
            HorizontalResult::Skip | HorizontalResult::Fail => {
                af.backtrack_from_checkpoint(checkpoint)
                    .next_opt(self.contents_wrap_to_fit_fn_opt(af))
                    .otherwise(|| self.contents_vertical(af))?;
            }
            HorizontalResult::Ok { height: 1 } => {}
            HorizontalResult::Ok { .. }
                if self.rest.is_none()
                    && ItemConfig::last_item_prefers_overflow(self.list.last().unwrap()) =>
            {}
            HorizontalResult::Ok {
                height: overflow_height,
            } => {
                // todo don't lookahead if there isn't any width gained by wrapping
                let overflow_lookahead = af.out.capture_lookahead(&checkpoint);

                // try vertical
                if af
                    .out
                    .with_enforce_max_width(|| self.contents_vertical(af))
                    .is_err()
                {
                    // separate lines failed, so overflow it is!
                    af.out.restore_checkpoint(&checkpoint);
                    af.out.restore_lookahead(overflow_lookahead);
                    return Ok(());
                }

                // choose between separate lines and overflow
                let vertical_height = af.out.line() - first_line + 1;
                if overflow_height < vertical_height {
                    af.out.restore_checkpoint(&checkpoint);
                    af.out.restore_lookahead(overflow_lookahead);
                }
            }
        }
        Ok(())
    }

    fn contents_wrap_to_fit_fn_opt(&self, af: &AstFormatter) -> Option<impl Fn() -> FormatResult> {
        if self.list.len() <= 1 && self.rest.is_none() {
            return None;
        }
        match Config::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                assert!(
                    self.rest.is_none(),
                    "rest cannot be used with wrap-to-fit"
                );
                Some(move || self.contents_wrap_to_fit(af, max_element_width))
            }
            ListWrapToFitConfig::No => None,
        }
    }

    fn contents_horizontal(&self, af: &AstFormatter) -> FormatResult {
        let len = self.list.len();
        af.list_contents_horizontal(
            len,
            |index, tail| {
                let strategy = ListStrategy::Horizontal;
                self.format_item.format(
                    af,
                    &self.list[index],
                    tail,
                    ListItemContext {
                        len,
                        index,
                        strategy,
                    },
                )
            },
            self.rest,
            self.braces.end,
            self.config.force_trailing_comma(),
            self.braces.pad,
            self.single_line_max_contents_width,
        )
    }

    fn contents_wrap_to_fit(
        &self,
        af: &AstFormatter,
        max_element_width: Option<HPos>,
    ) -> FormatResult {
        let len = self.list.len();
        let strategy = ListStrategy::WrapToFit;
        af.list_contents_wrap_to_fit(
            len,
            self.braces.end,
            self.tail,
            |index| {
                self.format_item.format(
                    af,
                    &self.list[index],
                    Tail::none(),
                    ListItemContext {
                        len,
                        index,
                        strategy,
                    },
                )
            },
            |index| ItemConfig::item_requires_own_line(&self.list[index]),
            max_element_width,
        )
    }

    fn contents_vertical(&self, af: &AstFormatter) -> FormatResult {
        let len = self.list.len();
        let strategy = ListStrategy::Vertical;
        af.list_contents_vertical(
            len,
            |index, tail| {
                self.format_item.format(
                    af,
                    &self.list[index],
                    tail,
                    ListItemContext {
                        len,
                        index,
                        strategy,
                    },
                )
            },
            self.rest,
            self.braces.end,
            self.tail,
        )
    }
}
