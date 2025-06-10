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
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
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
                // trying to not fallback to vertical because we need to see the width of the line when we fail because overflow is not allowed
                ListShape::Flexible => self.list_flexible(false)?,
                ListShape::FlexibleWithOverflow => self.list_flexible(true)?,
                ListShape::Horizontal => {
                    self.list_horizontal(false)?;
                }
                ListShape::HorizontalWithOverflow => {
                    self.list_horizontal(true)?;
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
        let wrap_to_fit_or_vertical = || {
            self.af
                .backtrack()
                .next_opt(self.list_wrap_to_fit_fn_opt())
                .next(|| self.list_vertical())
                .result_with_checkpoint(&checkpoint, true)?;
            Ok(())
        };
        if self.opt.item_requires_own_line.as_ref().is_some_and(|f| {
            self.list.iter().any(f)
        }) {
            return wrap_to_fit_or_vertical();
        }

        let horizontal_result = self.af.out.with_recover_width(|| self.list_horizontal(overflow));

        let horizontal_height = match horizontal_result {
            Ok(1) => return Ok(()),
            Ok(height) => height,
            Err(mut e) if let FormatErrorKind::ListItemOverflow { cause } = e.kind => {
                assert!(self.af.constraints().single_line.get(), "ListItemOverflow should only occur in single line mode");
                e.kind = FormatErrorKind::ListOverflow { cause };
                // Horizontal formatting would have succeeded if single line mode were not enabled.
                return Err(e);
            }
            Err(_) => return wrap_to_fit_or_vertical(),
        };

        if self.opt.rest.is_none()
            && self.opt.item_prefers_overflow.as_ref().is_some_and(|f| {
                f(self.list.last().unwrap())
            })
        {
            return Ok(());
        }

        // todo don't lookahead if there isn't any width gained by wrapping
        let horizontal_lookahead = self.af.out.capture_lookahead(&checkpoint);
        let vertical_result =
            self.af.out.with_recover_width(|| -> FormatResult<_> { self.list_vertical() });
        let use_vertical = vertical_result
            .is_ok_and(|height| height <= horizontal_height);
        if !use_vertical {
            self.af.out.restore_checkpoint(&checkpoint);
            self.af.out.restore_lookahead(horizontal_lookahead);
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

    fn list_horizontal(&self, overflow: bool) -> FormatResult<VSize> {
        let Self {
            af,
            opt,
            braces,
            list,
            format_item,
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
        let item = |index: usize, tail: Tail| -> FormatResult {
            format_item(
                af,
                &list[index],
                tail,
                ListItemContext {
                    index,
                    strategy: ListStrategy::Horizontal,
                },
            )
        };
        let height = Cell::new(0);
        let close = |af: &AstFormatter| {
            pad(af)?;
            af.out.token(braces.end())?;
            // N.B. measure before writing the tail
            height.set(af.out.line() - first_line + 1);
            af.tail(opt.tail)?;
            Ok(())
        };
        pad(af)?;
        // N.B. tails are created outside of width limit
        let list_item_tail = af.tail_fn(move |af| {
            if !rest.is_none() || opt.force_trailing_comma {
                af.out.token(",")?;
            } else {
                af.out.skip_token_if_present(",")?;
            }
            if rest.is_none() {
                close(af)?;
            }
            Ok(())
        });
        let last_item_tail = list_item_tail.as_ref();
        let end_tail = af.tail_fn(close);
        let end_tail = end_tail.as_ref();
        af.with_width_limit_opt(opt.single_line_max_contents_width, move || {
            if len == 0 {
                if let Some(rest) = rest {
                    list_rest(af, rest, end_tail)?;
                }
                return Ok(());
            }
            let (until_last, last) = (0..(len - 1), len - 1);
            for index in until_last {
                af.with_single_line(|| item(index, None))?;
                af.out.token_maybe_missing(",")?;
                af.out.space()?;
            }
            if overflow {
                // todo use simulate_wrap_indent to determine if vertical *might* be shorter
                item(last, last_item_tail).map_err(|mut err| {
                    match err.kind {
                        // We formatted the first line and stopped because we are in single-line
                        // mode, otherwise we could have continued without error.
                        FormatErrorKind::Vertical(cause) => {
                            assert!(self.af.constraints().single_line.get(), "FormatErrorKind::Vertical error should only occur in single line mode");
                            err.kind = FormatErrorKind::ListItemOverflow { cause };
                        }
                        // VStruct error - this list must be formatted vertically
                        // (it's here explicitly to show the necessity of this variant)
                        FormatErrorKind::VStruct { .. } => {}
                        _ => {}
                    }
                    err
                })?;
            } else {
                af.with_single_line(|| item(last, last_item_tail))?;
            }
            if let Some(rest) = rest {
                af.out.space()?;
                list_rest(af, rest, end_tail)?;
            }
            Ok(())
        })?;
        Ok(height.get())
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
                af.tail_fn(|af| af.out.token_maybe_missing(",")).as_ref(),
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
