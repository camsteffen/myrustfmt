use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn indent(&self) {
        self.out.indent()
    }

    pub fn newline_between(&self) -> FormatResult {
        self.out.newline(VerticalWhitespaceMode::Between)
    }

    pub fn newline_between_indent(&self) -> FormatResult {
        self.newline_between()?;
        self.indent();
        Ok(())
    }

    pub fn newline_top(&self) -> FormatResult {
        self.out.newline(VerticalWhitespaceMode::Top)
    }

    pub fn newline_top_if_comments(&self) -> FormatResult {
        self.out.newline_if_comments(VerticalWhitespaceMode::Top)
    }

    pub fn newline_bottom(&self) -> FormatResult {
        self.out.newline(VerticalWhitespaceMode::Bottom)
    }

    pub fn newline_break(&self) -> FormatResult {
        self.out.newline(VerticalWhitespaceMode::Break)
    }

    pub fn newline_break_indent(&self) -> FormatResult {
        self.newline_break()?;
        self.indent();
        Ok(())
    }

    pub fn newline_break_if_comments(&self) -> FormatResult {
        self.out.newline_if_comments(VerticalWhitespaceMode::Break)
    }
}
