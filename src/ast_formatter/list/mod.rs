mod list_item_context;
pub mod options;
mod rest;

pub use self::list_item_context::ListItemContext;
pub use self::rest::ListRest;
use crate::Recover;
use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::brackets::Brackets;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies, VerticalListStrategy,
    WrapToFit,
};
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::enclosed::ENCLOSED_DISALLOWED_VSTRUCTS;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::VSize;
use crate::source_formatter::checkpoint::Checkpoint;
use crate::util::cell_ext::CellNumberExt;
use crate::whitespace::VerticalWhitespaceMode;
use std::cell::Cell;

impl AstFormatter {
    pub fn list<'ast, Item>(
        &self,
        brackets: Brackets,
        list: &'ast [Item],
        format_item: impl Fn(&AstFormatter, &Item, Tail, ListItemContext) -> FormatResult,
        options: ListOptions<'ast, '_, Item>,
    ) -> FormatResult {
        ListContext {
            af: self,
            opt: options,
            format_item,
            brackets,
            list,
        }
        .format()
    }
}

struct ListContext<'af, 'ast, 'tail, Item, FormatItem> {
    af: &'af AstFormatter,
    opt: ListOptions<'ast, 'tail, Item>,
    brackets: Brackets,
    list: &'ast [Item],
    format_item: FormatItem,
}

impl<'af, 'ast, 'tail, Item, FormatItem> ListContext<'af, 'ast, 'tail, Item, FormatItem>
where
    FormatItem: Fn(&AstFormatter, &Item, Tail, ListItemContext) -> FormatResult,
{
    fn format(&self) -> FormatResult {
        self.af.has_vstruct(VStruct::List, || {
            if !self.opt.omit_open_bracket {
                // token_replace is for macro calls where we correct the bracket type
                self.af.out.token_replace(self.brackets.start())?;
            }
            if self.list.is_empty() && self.opt.rest.is_none() {
                self.af.enclosed_empty_contents()?;
                self.af.out.token_replace(self.brackets.end())?;
                self.af.tail(self.opt.tail)?;
                return Ok(());
            }
            match self.opt.strategies {
                ListStrategies::Horizontal(horizontal) => {
                    self.list_horizontal(horizontal, &Recover::default(), false)?
                }
                ListStrategies::Vertical(_) => {
                    self.list_vertical(None)?;
                }
                ListStrategies::Flexible(ref flexible) => self.list_flexible(flexible)?,
            }
            Ok(())
        })
    }

    fn list_flexible(&self, strategy: &FlexibleListStrategy<Item>) -> FormatResult {
        let checkpoint = self.af.out.checkpoint();

        if strategy
            .vertical
            .item_requires_own_line
            .as_ref()
            .is_some_and(|f| self.list.iter().any(f))
        {
            return self.list_vertical(Some(&checkpoint));
        }

        let recover = Recover::default();
        let horizontal_result = self.af.out.with_recover_width(|| {
            self.list_horizontal(strategy.horizontal, &recover, true)
        });
        if let Err(e) = horizontal_result {
            let recovering = recover.get()
                || match e.kind {
                    FormatErrorKind::Logical | FormatErrorKind::WidthLimitExceeded => true,
                    FormatErrorKind::VStruct { vstruct, .. } => {
                        !ENCLOSED_DISALLOWED_VSTRUCTS.contains(vstruct)
                    }
                    _ => false,
                };
            if recovering {
                self.af.out.restore_checkpoint(&checkpoint);
                self.list_vertical(Some(&checkpoint))?;
                return Ok(());
            }
            return Err(e);
        }

        Ok(())
    }

    fn list_horizontal(
        &self,
        strategy: HorizontalListStrategy,
        recover: &Recover,
        has_vertical_fallback: bool,
    ) -> FormatResult {
        let Self {
            af,
            opt,
            brackets,
            list,
            ..
        } = self;
        let rest = opt.rest;
        let len = list.len();
        let first_line = self.af.out.line();
        let _guard = self.af.constraints().version.increment_guard();
        let version = self.af.constraints().version.get();
        let item = |index, tail| self.list_item(index, Some(recover), tail);
        let pad = |af: &AstFormatter| -> FormatResult {
            if brackets.pad() {
                af.out.space()?;
            }
            Ok(())
        };
        let height = Cell::new(0);
        let close = |af: &AstFormatter| {
            pad(af)?;
            af.out.token_replace(brackets.end())?;
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
        let mut is_overflow_err = false;
        // structs are subject to width limit even with only one item
        let width_limit = if opt.is_struct
            || len + usize::from(rest.is_some_and(|r| r.base.is_some())) > 1
        {
            strategy.contents_max_width
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
                af.with_single_line(|| item(index, None))?;
                af.out.token_maybe_missing(",")?;
                af.out.space()?;
            }

            if strategy.overflow {
                let result = if has_vertical_fallback {
                    self.list_horizontal_overflowable(&height, recover, Some(&last_item_tail))
                } else {
                    item(list.len() - 1, Some(&last_item_tail))
                };
                result.inspect_err(|_| {
                    is_overflow_err = true;
                })?;
            } else {
                af.with_single_line(|| item(list.len() - 1, Some(&last_item_tail)))?;
            }
            if let Some(rest) = rest {
                af.out.space()?;
                list_rest(af, rest, Some(&end_tail))?;
            }
            Ok(())
        })
        .inspect_err(|e| {
            if let FormatErrorKind::Vertical(_) = e.kind
                && e.context_version >= version
                && !is_overflow_err
            {
                recover.set(true);
            }
        })?;
        Ok(())
    }

    fn list_horizontal_overflowable(
        &self,
        height: &Cell<VSize>,
        recover: &Recover,
        tail: Tail,
    ) -> FormatResult {
        let Self { af, list, .. } = self;
        let index = list.len() - 1;
        let item = || self.list_item(index, Some(recover), tail);
        let checkpoint = af.out.checkpoint();
        let wrap_result = af.simulate_wrap_indent(0, item)?;
        match wrap_result {
            SimulateWrapResult::Ok => {}
            SimulateWrapResult::NoWrap
            // vertical would increase indentation in the overflow
            | SimulateWrapResult::WrapForLongerFirstLine => {
                af.out.restore_checkpoint(&checkpoint);
                item()?;
            }
            SimulateWrapResult::WrapForSingleLine => {
                af.out.restore_checkpoint(&checkpoint);
                item()?;
                let vertical_height = 2 + list.len() as VSize;
                if vertical_height < height.get() {
                    return Err(self.af.err(FormatErrorKind::Logical));
                }
            }
            SimulateWrapResult::WrapForLessExcessWidth => {
                return Err(self.af.err(FormatErrorKind::Logical));
            }
        }
        Ok(())
    }

    fn list_wrap_to_fit_fn_opt<'a>(&'a self) -> Option<impl Fn(&Recover) -> FormatResult + 'a> {
        if self.list.len() <= 1 {
            return None;
        }
        let vertical = self.opt.strategies.get_vertical()?;
        let wrap_to_fit = vertical.wrap_to_fit?;
        assert!(
            self.opt.rest.is_none(),
            "rest cannot be used with wrap-to-fit",
        );
        Some(move |_: &Recover| self.list_wrap_to_fit(wrap_to_fit, vertical))
    }

    /*
    [
        item, item, item,
        item,
    ]
    */
    // todo how does this behave with comments between items - forcing newlines?
    fn list_wrap_to_fit(
        &self,
        wrap_to_fit: WrapToFit,
        vertical: &VerticalListStrategy<Item>,
    ) -> FormatResult {
        let WrapToFit {
            format_string_pos,
            max_element_width,
        } = wrap_to_fit;
        let Self {
            af,
            opt,
            brackets,
            list,
            ..
        } = self;
        let item = |index| {
            let item = || self.list_item(index, None, None);
            if let Some(max_width) = max_element_width
                && !format_string_pos.is_some_and(|i| index == i as usize)
            {
                af.with_single_line(|| af.with_width_limit(max_width.get(), item))
            } else {
                item()
            }
        };
        af.enclosed_contents(|| {
            let (first, rest) = (0, 1..list.len());
            item(first)?;
            af.out.token_maybe_missing(",")?;
            let mut prev_must_have_own_line = false;
            for index in rest {
                let item_comma = || -> FormatResult {
                    item(index)?;
                    af.out.token_maybe_missing(",")?;
                    Ok(())
                };
                let is_own_line = prev_must_have_own_line
                    || format_string_pos.is_some_and(|i| index == i as usize)
                    || vertical.item_requires_own_line.as_ref().is_some_and(|f| {
                        f(&list[index])
                    });
                if is_own_line {
                    prev_must_have_own_line = !prev_must_have_own_line;
                }
                af.backtrack()
                    .next_if(!is_own_line, |_| {
                        af.out.with_recover_width(|| {
                            af.out.space()?;
                            item_comma()?;
                            Ok(())
                        })
                    })
                    .next(|_| {
                        af.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        item_comma()?;
                        Ok(())
                    })
                    .result()?;
            }
            Ok(())
        })?;
        af.out.token_replace(brackets.end())?;
        af.tail(opt.tail)?;
        Ok(())
    }

    fn list_vertical(&self, checkpoint: Option<&Checkpoint>) -> FormatResult {
        self.af
            .backtrack()
            .next_opt(self.list_wrap_to_fit_fn_opt())
            .next(|_| self.list_vertical_simple())
            .result_opt_checkpoint(checkpoint)
    }

    fn list_vertical_simple(&self) -> FormatResult {
        let Self {
            af,
            opt,
            brackets,
            list,
            ..
        } = self;
        let len = list.len();
        let item_comma = |index| {
            self.list_item(
                index,
                None,
                Some(&af.tail_fn(|af| af.out.token_maybe_missing(","))),
            )
        };
        af.enclosed_contents(|| {
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
        af.out.token_replace(brackets.end())?;
        af.tail(opt.tail)?;
        Ok(())
    }

    fn list_item(&self, index: usize, horizontal: Option<&Recover>, tail: Tail) -> FormatResult {
        (self.format_item)(
            self.af,
            &self.list[index],
            tail,
            ListItemContext { horizontal, index },
        )
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
