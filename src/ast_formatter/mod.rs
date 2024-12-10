use crate::constraints::Constraints;
use crate::source_formatter::{FormatResult, SourceFormatter};

mod block;
mod common;
mod expr;
mod fallback_chain;
mod r#fn;
mod item;
pub mod list;
mod local;
mod pat;
mod ty;
mod qpath;

pub struct AstFormatter<'a> {
    out: SourceFormatter<'a>,
}

impl<'a> AstFormatter<'a> {
    pub fn new(out: SourceFormatter<'a>) -> Self {
        AstFormatter { out }
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn crate_(&mut self, crate_: &rustc_ast::ast::Crate) -> FormatResult {
        for item in &crate_.items {
            self.item(item)?;
        }
        Ok(())
    }

    fn constraints(&mut self) -> &mut Constraints {
        self.out.constraints()
    }

    fn with_indent(&mut self, f: impl FnOnce(&mut Self) -> FormatResult) -> FormatResult {
        self.constraints().increment_indent();
        let result = f(self);
        self.constraints().decrement_indent();
        result
    }

    fn with_single_line(&mut self, f: impl FnOnce(&mut Self) -> FormatResult) -> FormatResult {
        let single_line_prev = std::mem::replace(&mut self.constraints().single_line, true);
        let result = f(self);
        self.constraints().single_line = single_line_prev;
        result
    }

    fn with_reserved_width(
        &mut self,
        len: usize,
        f: impl FnOnce(&mut Self) -> FormatResult,
    ) -> FormatResult {
        self.constraints()
            .sub_max_width(len)
            .map_err(|e| self.out.lift_constraint_err(e))?;
        let result = f(self);
        self.constraints().add_max_width(len);
        result
    }

    fn with_width_limit_single_line(
        &mut self,
        width_limit: usize,
        f: impl FnOnce(&mut Self) -> FormatResult,
    ) -> FormatResult {
        let mut max_width = self.out.last_line_width() + width_limit;
        if let Some(current_max_width) = self.constraints().max_width {
            max_width = max_width.min(current_max_width);
        }
        let max_width_prev = std::mem::replace(&mut self.constraints().max_width, Some(max_width));
        let result = self.with_single_line(f);
        self.constraints().max_width = max_width_prev;
        result
    }
}
