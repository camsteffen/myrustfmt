mod braces;
pub mod builder;
pub mod list_config;
mod list_item_config;
mod list_item_context;
mod overflow;
mod rest;

pub use braces::Braces;
pub use list_item_config::ListItemConfig;
pub use list_item_context::{ListItemContext, ListStrategy};
pub use rest::ListRest;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use overflow::ListOverflow;

impl AstFormatter {
    /* [item, item, item] */
    fn list_contents_single_line<Item, Overflow: ListOverflow<Item = Item>>(
        &self,
        list: &[Item],
        format_item: impl Fn(&AstFormatter, &Item, &ListItemContext) -> FormatResult,
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
                for (i, item) in until_last.iter().enumerate() {
                    format_item(self, item, &ListItemContext {
                        index: i,
                        len: list.len(),
                        strategy: ListStrategy::SingleLine,
                    })?;
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
                    format_item(self, last, &ListItemContext {
                        index: list.len() - 1,
                        len: list.len(),
                        strategy: ListStrategy::SingleLine,
                    })?;
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
            if dbg!(can_overflow) {
                self.backtrack().next(last_without_overflow).otherwise(|| {
                    self.with_width_limit_from_start_first_line_opt(
                        start,
                        max_width_overflow,
                        || {
                            format_item(self, last, &ListItemContext {
                                index: list.len() - 1,
                                len: list.len(),
                                strategy: ListStrategy::SingleLine,
                            })?;
                            // Overflow::format_overflow(self, last)?;
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
        format_item: impl Fn(&AstFormatter, &T, &ListItemContext) -> FormatResult,
        _item_config: ItemConfig,
        max_element_width: Option<u32>,
    ) -> FormatResult
    where
        ItemConfig: ListItemConfig<Item = T>,
    {
        let format_item = |index, item| {
            let lcx = ListItemContext {
                index,
                len: list.len(),
                strategy: ListStrategy::WrapToFit,
            };
            match max_element_width {
                Some(max_width) => {
                    self.with_single_line_and_width_limit(max_width, || format_item(self, item, &lcx))
                }
                None => format_item(self, item, &lcx),
            }
        };
        self.embraced_after_opening(close_brace, || {
            let (first, rest) = list.split_first().unwrap();
            format_item(0, first)?;
            self.out.token_maybe_missing(",")?;
            let mut prev_must_have_own_line = false;
            for (i, item) in rest.iter().enumerate() {
                let index = i + 1;
                let item_comma = || -> FormatResult {
                    format_item(index, item)?;
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
                    self.backtrack()
                        .next(item_same_line)
                        .otherwise(item_next_line)?;
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
        format_item: impl Fn(&AstFormatter, &T, &ListItemContext) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        tail: &Tail,
    ) -> FormatResult {
        let item_comma = |index, item| -> FormatResult {
            format_item(self, item, &ListItemContext {
                index,
                len: list.len(),
                strategy: ListStrategy::SeparateLines,
            })?;
            self.out.token_maybe_missing(",")?;
            Ok(())
        };
        self.embraced_after_opening(close_brace, || {
            match rest {
                ListRest::None => {
                    let (last, until_last) = list.split_last().unwrap();
                    for (i, item) in until_last.iter().enumerate() {
                        item_comma(i, item)?;
                        // todo should this be "between"?
                        self.out.newline_within_indent()?;
                    }
                    item_comma(list.len() - 1, last)?;
                }
                _ => {
                    for (i, item) in list.iter().enumerate() {
                        item_comma(i, item)?;
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
