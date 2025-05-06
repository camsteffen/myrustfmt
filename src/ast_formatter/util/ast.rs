use rustc_ast::ast;
use rustc_span::BytePos;

pub fn item_lo_with_attrs(item: &ast::Item) -> BytePos {
    item.attrs.first().map_or(item.span, |a| a.span).lo()
}
