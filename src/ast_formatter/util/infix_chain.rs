use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn infix_chain<T>(
        &self,
        token: &str,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        should_indent: bool,
    ) -> FormatResult {
        let (first, rest) = items.split_first().unwrap();
        self.fallback(|| {
            self.with_single_line(|| {
                format_item(first)?;
                for item in rest {
                    self.out.space_token_space(token)?;
                    format_item(item)?;
                }
                Ok(())
            })
        })
        .otherwise(|| {
            format_item(first)?;
            self.indented_optional(should_indent, || {
                for item in rest {
                    self.out.newline_within_indent()?;
                    self.out.token_space(token)?;
                    format_item(item)?;
                }
                Ok(())
            })?;
            Ok(())
        })
    }
}
