use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn simple_infix_chain<T>(
        &self,
        token: &str,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        should_indent: bool,
        tail: &Tail,
    ) -> FormatResult {
        let (first, rest) = items.split_first().unwrap();
        self.backtrack()
            .next_single_line(|| {
                format_item(first)?;
                for item in rest {
                    self.out.space_token_space(token)?;
                    format_item(item)?;
                }
                self.tail(tail)?;
                Ok(())
            })
            .otherwise(|| {
                format_item(first)?;
                self.indented_optional(should_indent, || {
                    for item in rest {
                        self.out.newline_within_indent()?;
                        self.out.token_space(token)?;
                        format_item(item)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })?;
                Ok(())
            })
    }
}
