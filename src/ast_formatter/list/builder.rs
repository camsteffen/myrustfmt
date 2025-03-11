use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::{DefaultListConfig, ListConfig, ListWrapToFitConfig};
use crate::ast_formatter::list::list_item_config::DefaultListItemConfig;
use crate::ast_formatter::list::list_item_context::ListItemContext;
use crate::ast_formatter::list::{Braces, ListItemConfig, ListRest, ListStrategy};
use crate::ast_formatter::tail::Tail;
use crate::constraints::MultiLineShape;
use crate::error::{ConstraintError, FormatResult, FormatResultExt};

macro_rules! return_if_ok {
    ($expr:expr) => {
        match $expr {
            Ok(value) => return Ok(value),
            Err(e) => e,
        }
    };
}

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
    single_line_max_contents_width: Option<u32>,
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

    pub fn single_line_max_contents_width(self, width: u32) -> Self {
        ListBuilder {
            single_line_max_contents_width: Some(width),
            ..self
        }
    }

    pub fn format(&self, af: &AstFormatter) -> FormatResult {
        af.constraints()
            .with_single_line_unless(MultiLineShape::VerticalList, || {
                self.do_format(af, Self::contents_default)
            })
    }

    pub fn format_single_line(&self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_single_line)?;
        af.tail(self.tail)?;
        Ok(())
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
        if self.list.is_empty() && self.rest.is_none() {
            af.embraced_empty_after_opening(self.braces.end)?;
            af.tail(self.tail)?;
            return Ok(());
        }
        contents(self, af)
    }

    // todo better name
    fn contents_default(&self, af: &AstFormatter) -> FormatResult {
        let checkpoint = af.out.checkpoint();
        let first_line = af.out.line();
        let mut dirty = false;
        let overflow_lookahead = 'lookahead: {
            let any_items_require_own_line =
                self.list.iter().any(ItemConfig::item_requires_own_line);
            if any_items_require_own_line {
                break 'lookahead None;
            }
            dirty = true;
            let enforce_max_width_guard = af.out.enforce_max_width();
            if self
                .contents_single_line(af)
                .constraint_err_only()?
                .is_err()
            {
                break 'lookahead None;
            }
            if (
                self.rest.is_none()
                    && ItemConfig::last_item_prefers_overflow(self.list.last().unwrap())
            ) || af.out.line() == first_line
            {
                let _: ConstraintError = return_if_ok!(af.tail(self.tail).constraint_err_only()?);
                break 'lookahead None;
            }
            // todo don't lookahead if there isn't any width gained by wrapping
            let overflow_height = af.out.line() - first_line + 1;
            if af.tail(self.tail).constraint_err_only()?.is_err() {
                break 'lookahead None;
            }
            drop(enforce_max_width_guard);
            let lookahead = af.out.capture_lookahead(&checkpoint);
            Some((lookahead, overflow_height))
        };
        let Some((overflow_lookahead, overflow_height)) = overflow_lookahead else {
            if dirty {
                af.out.restore_checkpoint(&checkpoint);
            }
            return af
                .backtrack_from_checkpoint(checkpoint)
                .next_opt(self.contents_wrap_to_fit_fn_opt(af))
                .otherwise(|| self.contents_separate_lines(af));
        };
        if let Some(f) = self.contents_wrap_to_fit_fn_opt(af) {
            let _: ConstraintError =
                return_if_ok!(af.out.with_enforce_max_width(f).constraint_err_only()?);
            af.out.restore_checkpoint(&checkpoint);
        }
        match af
            .out
            .with_enforce_max_width(|| self.contents_separate_lines(af))
            .constraint_err_only()?
        {
            Err(_) => {
                af.out.restore_checkpoint(&checkpoint);
                af.out.restore_lookahead(&overflow_lookahead);
            }
            Ok(()) => {
                let separate_lines_height = af.out.line() - first_line + 1;
                if overflow_height < separate_lines_height {
                    af.out.restore_checkpoint(&checkpoint);
                    af.out.restore_lookahead(&overflow_lookahead);
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

    fn contents_single_line(&self, af: &AstFormatter) -> FormatResult {
        let len = self.list.len();
        af.list_contents_single_line(
            len,
            |index, tail| {
                let strategy = ListStrategy::SingleLine;
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
        max_element_width: Option<u32>,
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

    fn contents_separate_lines(&self, af: &AstFormatter) -> FormatResult {
        let len = self.list.len();
        let strategy = ListStrategy::SeparateLines;
        af.list_contents_separate_lines(
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
