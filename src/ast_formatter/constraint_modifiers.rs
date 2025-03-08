use std::rc::Rc;
use crate::ast_formatter::AstFormatter;
use crate::constraints::{CheckpointCounter, MaxWidthForLine, MultiLineShape, OwnedConstraints};
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::util::cell_ext::CellExt;

pub const INDENT_WIDTH: u32 = 4;

impl AstFormatter {
    pub(super) fn checkpoint_counter(&self) -> &Rc<CheckpointCounter> {
        self.out.checkpoint_counter()
    }

    pub(super) fn constraints(&self) -> &OwnedConstraints {
        self.out.constraints()
    }

    pub fn indented<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let indent = self.out.indent.get() + INDENT_WIDTH;
        self.out.indent.with_replaced(indent, || {
            let shape = self.constraints().borrow().multi_line;
            match shape {
                MultiLineShape::SingleLine | MultiLineShape::Unrestricted => f(),
                _ => {
                    self.constraints()
                        .with_multi_line_shape_replaced(MultiLineShape::Unrestricted, f)
                }
            }
        })
    }

    pub fn indented_optional(
        &self,
        should_indent: bool,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !should_indent {
            return f();
        }
        self.indented(f)
    }

    pub fn with_single_line<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        assert!(
            self.checkpoint_counter().count() > 0,
            "single line constraint applied with no fallback"
        );
        self.constraints()
            .with_multi_line_shape_replaced(MultiLineShape::SingleLine, f)
    }

    /** Enforces a max number of characters until a newline is printed */
    pub fn with_width_limit_first_line<T>(&self, width_limit: u32, f: impl FnOnce() -> T) -> T {
        let line = self.out.line();
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .constraints()
            .borrow()
            .max_width
            .is_some_and(|mw| max_width >= mw)
            || self
                .constraints()
                .borrow()
                .max_width_for_line
                .is_some_and(|m| m.line == line && m.max_width <= max_width)
        {
            return f();
        }
        self.constraints()
            .with_max_width_for_line(Some(MaxWidthForLine { line, max_width }), f)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, f),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.last_line_len() - line_start_pos)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(remaining, f)
    }

    pub fn with_width_limit_from_start_first_line<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.last_line_len() - line_start_pos)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, f)
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        line_start_pos: u32,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        self.with_width_limit_from_start_first_line(line_start_pos, width_limit, f)
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .constraints()
            .borrow()
            .max_width
            .is_some_and(|m| m <= max_width)
        {
            f()
        } else {
            self.constraints().with_max_width(Some(max_width), f)
        }
    }
}
