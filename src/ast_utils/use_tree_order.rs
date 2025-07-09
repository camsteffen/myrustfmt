use crate::ast_utils::version_sort::version_sort;
use crate::util::cmp::{cmp_by_key, cmp_iter_by};
use rustc_ast::ast;
use rustc_data_structures::fx::FxHashMap;
use rustc_span::BytePos;
use rustc_span::Symbol;
use rustc_span::kw;
use std::cmp::Ordering;

pub type SortedUseTreeMap = FxHashMap<BytePos, Vec<usize>>;

pub fn use_tree_order(a: &ast::UseTree, b: &ast::UseTree, sort_map: &SortedUseTreeMap) -> Ordering {
    cmp_iter_by(iter(a), iter(b), |element_a, element_b| {
        cmp_by_key(&element_a, &element_b, |e| match e {
            Element::Segment(_) => 0,
            Element::Glob => 1,
            Element::Nested(..) => 2,
        })
        .then_with(|| match (element_a, element_b) {
            (Element::Segment(a), Element::Segment(b)) => {
                match (segment_order_key(a), segment_order_key(b)) {
                    (SegmentOrderKey::Other, SegmentOrderKey::Other) => {
                        version_sort(a.as_str(), b.as_str())
                    }
                    (key_a, key_b) => key_a.cmp(&key_b),
                }
            }
            (Element::Glob, Element::Glob) => Ordering::Equal,
            (Element::Nested(a, a_pos), Element::Nested(b, b_pos)) => {
                let a_sorted = sort_map
                    .get(&a_pos)
                    .unwrap_or_else(|| panic!("sort_map is missing {a_pos:?}"));
                let b_sorted = sort_map
                    .get(&b_pos)
                    .unwrap_or_else(|| panic!("sort_map is missing {b_pos:?}"));
                cmp_iter_by(a_sorted, b_sorted, |&ai, &bi| {
                    use_tree_order(&a[ai].0, &b[bi].0, sort_map)
                })
            }
            _ => unreachable!(),
        })
    })
}

enum Element<'a> {
    Segment(Symbol),
    Glob,
    Nested(&'a [(ast::UseTree, ast::NodeId)], BytePos),
}

fn iter(use_tree: &ast::UseTree) -> impl Iterator<Item = Element<'_>> {
    let prefix = use_tree.prefix.segments.iter().map(|s| {
        Element::Segment(s.ident.name)
    });
    prefix.chain(std::iter::once(()).filter_map(|()| match &use_tree.kind {
        ast::UseTreeKind::Simple(_) => None,
        ast::UseTreeKind::Glob => Some(Element::Glob),
        ast::UseTreeKind::Nested { items, span } => Some(Element::Nested(items, span.lo())),
    }))
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum SegmentOrderKey {
    Self_,
    Super,
    Crate,
    PathRoot,
    Other,
}

fn segment_order_key(symbol: Symbol) -> SegmentOrderKey {
    match symbol {
        kw::SelfLower => SegmentOrderKey::Self_,
        kw::Super => SegmentOrderKey::Super,
        kw::Crate => SegmentOrderKey::Crate,
        kw::PathRoot => SegmentOrderKey::PathRoot,
        _ => SegmentOrderKey::Other,
    }
}
