mod braces;
mod list_item_context;
pub mod options;
mod rest;

pub use self::braces::Braces;
pub use self::list_item_context::{ListItemContext, ListStrategy};
pub use self::rest::ListRest;
use std::cell::Cell;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::options::{ListOptions, ListShape, ListWrapToFit};
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::constraints::VStruct;
use crate::error::{FormatError, FormatErrorKind, FormatResult};
use crate::num::{HSize, VSize};
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn list<'ast, Item>(
        &self,
        braces: Braces,
        list: &'ast [Item],
        format_item: impl Fn(&AstFormatter, &Item, Tail, ListItemContext) -> FormatResult,
        options: ListOptions<'ast, '_, Item>,
    ) -> FormatResult {
        ListContext {
            af: self,
            opt: options,
            format_item,
            braces,
            list,
        }
        .format()
    }
}

struct ListContext<'af, 'ast, 'tail, Item, FormatItem> {
    pub af: &'af AstFormatter,
    pub opt: ListOptions<'ast, 'tail, Item>,
    pub braces: Braces,
    pub list: &'ast [Item],
    pub format_item: FormatItem,
}

impl<'af, 'ast, 'tail, Item, FormatItem> ListContext<'af, 'ast, 'tail, Item, FormatItem>
where
    FormatItem: Fn(&AstFormatter, &Item, Tail, ListItemContext) -> FormatResult,
{
    fn format(&self) -> FormatResult {
        self.af.has_vstruct(VStruct::List, || {
            if !self.opt.omit_open_brace {
                self.af.out.token(self.braces.start())?;
            }
            if self.list.is_empty() && self.opt.rest.is_none() {
                self.af.enclosed_empty_after_opening(self.braces.end())?;
                self.af.tail(self.opt.tail)?;
                return Ok(());
            }
            match self.opt.shape {
                ListShape::Flexible => self.list_flexible(false)?,
                ListShape::FlexibleWithOverflow => self.list_flexible(true)?,
                ListShape::Horizontal => {
                    self.list_horizontal(false, false).map_err(|e| e.error)?;
                }
                ListShape::HorizontalWithOverflow => {
                    self.list_horizontal(true, false).map_err(|e| e.error)?;
                }
                ListShape::Vertical => {
                    self.list_vertical()?;
                }
            };
            Ok(())
        })
    }

    fn list_flexible(&self, overflow: bool) -> FormatResult {
        let checkpoint = self.af.out.checkpoint();

        let wrap_to_fit_or_vertical = |restore: bool| {
            self.af
                .backtrack()
                .next_opt(self.list_wrap_to_fit_fn_opt())
                .next(|| self.list_vertical())
                .result_with_checkpoint(&checkpoint, restore)?;
            Ok(())
        };

        if self.opt.item_requires_own_line.as_ref().is_some_and(|f| {
            self.list.iter().any(f)
        }) {
            return wrap_to_fit_or_vertical(false);
        }

        if let Err(e) = self.af.out.with_recover_width(|| self.list_horizontal(overflow, true)) {
            if e.is_overflow {
                assert!(self.af.constraints().single_line.get(), "list overflow error should only occur in single line mode");
                // Horizontal formatting would have succeeded if single line mode were not enabled.
                return Err(e.error);
            }
            return wrap_to_fit_or_vertical(true);
        }

        Ok(())
    }

    fn list_horizontal(&self, overflow: bool, flexible: bool) -> Result<(), ListHorizontalError> {
        let Self {
            af,
            opt,
            braces,
            list,
            ..
        } = self;
        let rest = opt.rest;
        let len = list.len();
        let first_line = self.af.out.line();
        let pad = |af: &AstFormatter| -> FormatResult {
            if braces.pad() {
                af.out.space()?;
            }
            Ok(())
        };
        let height = Cell::new(0);
        let close = |af: &AstFormatter| {
            pad(af)?;
            af.out.token(braces.end())?;
            // measure height before writing the tail
            height.set(af.out.line() - first_line + 1);
            af.tail(opt.tail)?;
            Ok(())
        };
        pad(af)?;
        // N.B. tails are created outside of width limit
        let end_tail = af.tail_fn(close);
        let last_item_tail = af.tail_fn(|af| {
            if rest.is_some() || opt.force_trailing_comma {
                af.out.token(",")?;
            } else {
                af.out.token_skip_if_present(",")?;
            }
            if rest.is_none() {
                close(af)?;
            }
            Ok(())
        });
        let mut is_overflow = false;
        // structs are subject to width limit even with only one item
        let width_limit = if opt.is_struct
            || len + usize::from(rest.is_some_and(|r| r.base.is_some())) > 1
        {
            opt.contents_max_width
        } else {
            None
        };
        af.with_width_limit_opt(width_limit, || {
            if len == 0 {
                if let Some(rest) = rest {
                    list_rest(af, rest, Some(&end_tail))?;
                }
                return Ok(());
            }

            for index in 0..(len - 1) {
                af.with_single_line(|| self.list_horizontal_item(index, None))?;
                af.out.token_maybe_missing(",")?;
                af.out.space()?;
            }

            let mut last_item_result = if !overflow {
                af.with_single_line(|| {
                    self.list_horizontal_item(list.len() - 1, Some(&last_item_tail))
                })
            } else if !flexible {
                self.list_horizontal_item(list.len() - 1, Some(&last_item_tail))
            } else {
                self.list_horizontal_overflowable(&height, Some(&last_item_tail))
            };
            if overflow
                && let Err(err) = &mut last_item_result
                && let FormatErrorKind::Vertical(_) = err.kind
            {
                // We formatted the first line and stopped because we are in single-line mode.
                // Otherwise, we could have continued without error.
                is_overflow = true;
            }
            last_item_result?;
            if let Some(rest) = rest {
                af.out.space()?;
                list_rest(af, rest, Some(&end_tail))?;
            }
            Ok(())
        })
        .map_err(|error| ListHorizontalError { error, is_overflow })?;
        Ok(())
    }

    fn list_horizontal_item(&self, index: usize, tail: Tail) -> FormatResult {
        (self.format_item)(
            self.af,
            &self.list[index],
            tail,
            ListItemContext {
                index,
                strategy: ListStrategy::Horizontal,
            },
        )
    }

    fn list_horizontal_overflowable(&self, height: &Cell<VSize>, tail: Tail) -> FormatResult {
        let Self { af, list, .. } = self;
        let index = list.len() - 1;
        let checkpoint = af.out.checkpoint();
        let wrap_result = af.simulate_wrap_indent(0, || self.list_horizontal_item(index, tail));
        match wrap_result {
            SimulateWrapResult::Ok => {}
            SimulateWrapResult::NoWrap
            // vertical would increase indentation in the overflow
            | SimulateWrapResult::WrapForLongerFirstLine => {
                af.out.restore_checkpoint(&checkpoint);
                self.list_horizontal_item(index, tail)?;
            }
            SimulateWrapResult::WrapForSingleLine => {
                af.out.restore_checkpoint(&checkpoint);
                self.list_horizontal_item(index, tail)?;
                let vertical_height = 2 + list.len() as VSize;
                if vertical_height < height.get() {
                    return Err(FormatErrorKind::Logical.into());
                }
            }
            SimulateWrapResult::WrapForLessExcessWidth => {
                return Err(FormatErrorKind::Logical.into());
            }
        }
        Ok(())
    }

    fn list_wrap_to_fit_fn_opt(&self) -> Option<impl Fn() -> FormatResult<VSize>> {
        if self.list.len() <= 1 && self.opt.rest.is_none() {
            return None;
        }
        match self.opt.wrap_to_fit {
            ListWrapToFit::Yes { max_element_width } => {
                assert!(
                    self.opt.rest.is_none(),
                    "rest cannot be used with wrap-to-fit"
                );
                Some(move || self.list_wrap_to_fit(max_element_width))
            }
            ListWrapToFit::No => None,
        }
    }

    /*
    [
        item, item, item,
        item,
    ]
    */
    // todo how does this behave with comments between items - forcing newlines?
    fn list_wrap_to_fit(&self, max_element_width: Option<HSize>) -> FormatResult<VSize> {
        let Self {
            af,
            opt,
            braces,
            list,
            ..
        } = self;
        let len = list.len();
        let first_line = af.out.line();
        let format_index = |index| {
            (self.format_item)(
                self.af,
                &self.list[index],
                None,
                ListItemContext {
                    index,
                    strategy: ListStrategy::WrapToFit,
                },
            )
        };
        let format_item = format_index;
        let format_item = |index| match max_element_width {
            Some(max_width) => {
                af.with_single_line(|| af.with_width_limit(max_width, || format_item(index)))
            }
            None => format_item(index),
        };
        af.enclosed_after_opening(braces.end(), || {
            let (first, rest) = (0, 1..len);
            format_item(first)?;
            af.out.token_maybe_missing(",")?;
            let mut prev_must_have_own_line = false;
            for index in rest {
                let item_comma = || -> FormatResult {
                    format_item(index)?;
                    af.out.token_maybe_missing(",")?;
                    Ok(())
                };
                let is_own_line = prev_must_have_own_line
                    || opt.item_requires_own_line.as_ref().is_some_and(|f| {
                        f(&list[index])
                    });
                if is_own_line {
                    prev_must_have_own_line = !prev_must_have_own_line;
                }
                af.backtrack()
                    .next_if(!is_own_line, || {
                        af.out.with_recover_width(|| {
                            af.out.space()?;
                            item_comma()?;
                            Ok(())
                        })
                    })
                    .next(|| {
                        af.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        item_comma()?;
                        Ok(())
                    })
                    .result()?;
            }
            Ok(())
        })?;
        let height = af.out.line() - first_line + 1;
        af.tail(opt.tail)?;
        Ok(height)
    }

    fn list_vertical(&self) -> FormatResult<VSize> {
        let Self {
            af,
            opt,
            braces,
            list,
            ..
        } = self;
        let len = list.len();
        let first_line = af.out.line();
        let item_comma = |index| {
            (self.format_item)(
                af,
                &list[index],
                Some(&af.tail_fn(|af| af.out.token_maybe_missing(","))),
                ListItemContext {
                    index,
                    strategy: ListStrategy::Vertical,
                },
            )
        };
        af.enclosed_after_opening(braces.end(), || {
            match opt.rest {
                None => {
                    for index in 0..len - 1 {
                        item_comma(index)?;
                        // todo should this be "between"?
                        af.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    }
                    item_comma(len - 1)?;
                }
                Some(rest) => {
                    for index in 0..len {
                        item_comma(index)?;
                        af.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    }
                    list_rest(af, rest, None)?;
                }
            }
            Ok(())
        })?;
        let height = af.out.line() - first_line + 1;
        af.tail(opt.tail)?;
        Ok(height)
    }
}

fn list_rest(af: &AstFormatter, rest: ListRest, tail: Tail) -> FormatResult {
    af.out.token("..")?;
    if let Some(expr) = rest.base {
        af.expr_tail(expr, tail)?;
    } else {
        af.tail(tail)?;
    }
    Ok(())
}

#[derive(Debug)]
struct ListHorizontalError {
    error: FormatError,
    /// True if the _current_ list is an overflow error
    is_overflow: bool,
}

impl From<FormatError> for ListHorizontalError {
    fn from(error: FormatError) -> Self {
        ListHorizontalError {
            error,
            is_overflow: false,
        }
    }
}
