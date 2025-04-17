use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};

impl AstFormatter {
    pub fn take_while_within_margin<'a, T>(
        &'a self,
        mut iter: impl Iterator<Item = T> + 'a,
    ) -> impl Iterator<Item = T> {
        let indent_margin = self.out.total_indent.get() + INDENT_WIDTH;
        std::iter::from_fn(move || {
            if self.out.last_line_len() < indent_margin {
                iter.next()
            } else {
                None
            }
        })
    }
}
