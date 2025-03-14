mod braces;
pub mod builder;
pub mod list_config;
mod list_item_config;
mod list_item_context;
mod rest;

pub use self::braces::Braces;
pub use self::list_item_config::ListItemConfig;
pub use self::list_item_context::{ListItemContext, ListStrategy};
pub use self::rest::ListRest;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::num::HPos;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    /* [item, item, item] */
    fn list_contents_horizontal(
        &self,
        len: usize,
        format_item: impl Fn(/*index: */ usize, &Tail) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        force_trailing_comma: bool,
        pad: bool,
        max_width: Option<HPos>,
    ) -> FormatResult {
        let do_pad = |af: &Self| -> FormatResult {
            if pad {
                af.out.space()?;
            }
            Ok(())
        };
        self.with_single_line(|| {
            do_pad(self)?;
            let close = |af: &Self| {
                do_pad(af)?;
                af.out.token(close_brace)?;
                Ok(())
            };
            // N.B. tails are created outside of width limit
            let close_tail = self.tail_fn(close);
            let last_tail = self.tail_fn(|af| {
                if !rest.is_none() || force_trailing_comma {
                    af.out.token(",")?;
                } else {
                    af.out.skip_token_if_present(",")?;
                }
                if rest.is_none() {
                    close(af)?;
                }
                Ok(())
            });
            self.with_width_limit_first_line_opt(max_width, || {
                if len == 0 {
                    if !rest.is_none() {
                        self.list_rest(rest, &close_tail)?;
                    }
                    return Ok(());
                }
                let (until_last, last) = (0..(len - 1), len - 1);
                for index in until_last {
                    format_item(index, Tail::none())?;
                    self.out.token_maybe_missing(",")?;
                    self.out.space()?;
                }
                // A tail is only necessary with the last item since it may overflow
                format_item(last, &last_tail)?;
                if !rest.is_none() {
                    self.out.space()?;
                    self.list_rest(rest, &close_tail)?;
                }
                Ok(())
            })
        })
    }

    /*
    [
        item, item, item,
        item,
    ]
    */
    // todo how does this behave with comments between items - forcing newlines?
    fn list_contents_wrap_to_fit(
        &self,
        len: usize,
        close_brace: &str,
        tail: &Tail,
        format_item: impl Fn(/*index: */ usize) -> FormatResult,
        item_requires_own_line: impl Fn(/*index: */ usize) -> bool,
        max_element_width: Option<HPos>,
    ) -> FormatResult {
        let format_item = |index| match max_element_width {
            Some(max_width) => {
                self.with_single_line(|| self.with_width_limit(max_width, || format_item(index)))
            }
            None => format_item(index),
        };
        self.embraced_after_opening(close_brace, || {
            let (first, rest) = (0, 1..len);
            format_item(first)?;
            self.out.token_maybe_missing(",")?;
            let mut prev_must_have_own_line = false;
            for index in rest {
                let item_comma = || -> FormatResult {
                    format_item(index)?;
                    self.out.token_maybe_missing(",")?;
                    Ok(())
                };
                let is_own_line = prev_must_have_own_line || item_requires_own_line(index);
                if is_own_line {
                    prev_must_have_own_line = !prev_must_have_own_line;
                }
                self.backtrack()
                    .next_if(!is_own_line, || {
                        self.out.space()?;
                        item_comma()?;
                        Ok(())
                    })
                    .otherwise(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        item_comma()?;
                        Ok(())
                    })?;
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
    fn list_contents_vertical(
        &self,
        len: usize,
        format_item: impl Fn(/*index: */ usize, &Tail) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        tail: &Tail,
    ) -> FormatResult {
        let comma = self.tail_fn(|af| af.out.token_maybe_missing(","));
        let item_comma = |index| format_item(index, &comma);
        self.embraced_after_opening(close_brace, || {
            match rest {
                ListRest::None => {
                    for index in 0..len - 1 {
                        item_comma(index)?;
                        // todo should this be "between"?
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    }
                    item_comma(len - 1)?;
                }
                _ => {
                    for index in 0..len {
                        item_comma(index)?;
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    }
                    self.list_rest(rest, Tail::none())?;
                }
            }
            Ok(())
        })?;
        self.tail(tail)?;
        Ok(())
    }

    fn list_rest(&self, rest: ListRest<'_>, tail: &Tail) -> FormatResult {
        self.out.token("..")?;
        if let ListRest::Base(expr) = rest {
            self.expr_tail(expr, tail)?;
        } else {
            self.tail(tail)?;
        }
        Ok(())
    }
}
