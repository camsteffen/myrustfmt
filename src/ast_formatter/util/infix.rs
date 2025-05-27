use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn simple_infix_chain<T>(
        &self,
        token: &'static str,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        should_indent: bool,
        tail: Tail,
    ) -> FormatResult {
        let (first, rest) = items.split_first().unwrap();
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    format_item(first)?;
                    for item in rest {
                        self.out.space_token_space(token)?;
                        format_item(item)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|| {
                format_item(first)?;
                self.indented_optional(should_indent, || {
                    for item in rest {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space(token)?;
                        format_item(item)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })?;
                Ok(())
            })
            .result()
    }
}
