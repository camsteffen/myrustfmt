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
                rest.iter().try_for_each(|item| -> FormatResult {
                    self.out.space_token_space(token)?;
                    format_item(item)?;
                    Ok(())
                })?;
                Ok(())
            })
        })
        .next(|| {
            format_item(first)?;
            self.indented_optional(should_indent, || {
                rest.iter().try_for_each(|item| {
                    self.out.newline_indent()?;
                    self.out.token_space(token)?;
                    format_item(item)?;
                    Ok(())
                })
            })?;
            Ok(())
        })
        .result()
    }
}
