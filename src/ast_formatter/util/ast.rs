use crate::ast_formatter::AstFormatter;
use rustc_ast::ast;
use rustc_span::BytePos;

pub fn item_lo_with_attrs(item: &ast::Item) -> BytePos {
    item.attrs.first().map_or(item.span, |a| a.span).lo()
}

impl AstFormatter {
    pub fn split_off_contiguous_group<'a, T>(
        &self,
        remaining: &mut &'a [T],
        filter: impl Fn(&T) -> bool,
        get_lo: impl Fn(&T) -> BytePos,
        get_hi: impl Fn(&T) -> BytePos,
    ) -> &'a [T] {
        let first = remaining.first().unwrap();
        let source_file = &self.out.source_reader.source_file;
        let mut line_hi = source_file
            .lookup_line(source_file.relative_position(get_hi(first)))
            .unwrap();
        let more_count = remaining[1..]
            .iter()
            .take_while(|item| {
                if !filter(item) {
                    return false;
                }
                let next_lo = source_file
                    .lookup_line(source_file.relative_position(get_lo(item)))
                    .unwrap();
                if next_lo - line_hi > 1 {
                    return false;
                }
                line_hi = source_file
                    .lookup_line(source_file.relative_position(get_hi(item)))
                    .unwrap();
                true
            })
            .count();
        remaining.split_off(..1 + more_count).unwrap()
    }
}
