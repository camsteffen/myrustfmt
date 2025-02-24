mod braces;
pub mod builder;
pub mod list_config;
mod list_item_config;
mod list_item_context;
mod rest;

pub use braces::Braces;
pub use list_item_config::ListItemConfig;
pub use list_item_context::{ListItemContext, ListStrategy};
pub use rest::ListRest;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;

impl AstFormatter {
    /* [item, item, item] */
    fn list_contents_single_line(
        &self,
        len: usize,
        format_item: impl Fn(/*index: */ usize, &Tail) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        force_trailing_comma: bool,
        pad: bool,
        max_width: Option<u32>,
    ) -> FormatResult {
        let do_pad = |af: &Self| -> FormatResult {
            if pad {
                af.out.space()?;
            }
            Ok(())
        };
        self.with_single_line(|| {
            do_pad(self)?;
            let prev_max_width_for_line = self.constraints().max_width_for_line.get();
            self.with_width_limit_first_line_opt(max_width, || {
                let close = |af: &Self| {
                    self.constraints()
                        .max_width_for_line
                        .with_replaced(prev_max_width_for_line, || {
                            do_pad(af)?;
                            af.out.token(close_brace)?;
                            Ok(())
                        })
                };
                if len == 0 {
                    if !rest.is_none() {
                        self.list_rest(rest, Tail::none())?;
                        close(self)?;
                    }
                    return Ok(());
                };
                let (until_last, last) = (0..(len - 1), len - 1);
                for index in until_last {
                    format_item(index, Tail::none())?;
                    self.out.token_maybe_missing(",")?;
                    self.out.space()?;
                }
                // A tail is only necessary with the last item since it may overflow
                format_item(
                    last,
                    &Tail::func(|af| {
                        if !rest.is_none() || force_trailing_comma {
                            af.out.token(",")?;
                        } else {
                            af.out.skip_token_if_present(",")?;
                        }
                        if rest.is_none() {
                            close(af)?
                        }
                        Ok(())
                    }),
                )?;
                if !rest.is_none() {
                    self.out.space()?;
                    self.list_rest(rest, &Tail::func(close))?;
                }
                Ok(())
            })?;
            Ok(())
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
        max_element_width: Option<u32>,
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
                        self.out.newline_within_indent()?;
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
    fn list_contents_separate_lines(
        &self,
        len: usize,
        format_item: impl Fn(/*index: */ usize, &Tail) -> FormatResult,
        rest: ListRest<'_>,
        close_brace: &str,
        tail: &Tail,
    ) -> FormatResult {
        let item_comma =
            |index| -> FormatResult { format_item(index, &Tail::token_maybe_missing(",")) };
        self.embraced_after_opening(close_brace, || {
            match rest {
                ListRest::None => {
                    for index in 0..len - 1 {
                        item_comma(index)?;
                        // todo should this be "between"?
                        self.out.newline_within_indent()?;
                    }
                    item_comma(len - 1)?;
                }
                _ => {
                    for index in 0..len {
                        item_comma(index)?;
                        self.out.newline_within_indent()?;
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
