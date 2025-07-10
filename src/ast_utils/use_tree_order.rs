use crate::ast_utils::version_sort::version_sort;
use crate::util::cmp::{cmp_by_key, cmp_iter_by};
use rustc_ast::ast;
use rustc_data_structures::fx::FxHashMap;
use rustc_span::BytePos;
use rustc_span::Span;
use rustc_span::Symbol;
use rustc_span::kw;
use std::cmp::Ordering;

pub type SortedUseTreeMap = FxHashMap<BytePos, Vec<usize>>;

pub fn get_sorted_use_tree<'a>(
    sort_map: &'a SortedUseTreeMap,
    items: &[(ast::UseTree, ast::NodeId)],
    span: Span,
) -> &'a [usize] {
    match items.len() {
        0 => &[],
        1 => &[0],
        _ => {
            sort_map
                .get(&span.lo())
                .unwrap_or_else(|| panic!("sort_map is missing {span:?}"))
        }
    }
}

pub fn use_tree_order(a: &ast::UseTree, b: &ast::UseTree, sort_map: &SortedUseTreeMap) -> Ordering {
    cmp_iter_by(iter_elements(a), iter_elements(b), |element_a, element_b| {
        use_tree_element_order(element_a, element_b, sort_map)
    })
}

fn use_tree_element_order(
    element_a: Element,
    element_b: Element,
    sort_map: &SortedUseTreeMap,
) -> Ordering {
    cmp_by_key(&element_a, &element_b, |e| match e {
        Element::Segment(_) => 0,
        Element::Glob => 1,
        Element::Nested(..) => 2,
    })
    .then_with(|| match (element_a, element_b) {
        (Element::Segment(a), Element::Segment(b)) => {
            cmp_by_key(a, b, |s| match s {
                kw::SelfLower => 0,
                kw::Super => 1,
                kw::Crate => 2,
                kw::PathRoot => 3,
                _ => 4,
            })
            .then_with(|| version_sort(a.as_str(), b.as_str()))
        }
        (Element::Glob, Element::Glob) => Ordering::Equal,
        (Element::Nested(a, a_span), Element::Nested(b, b_span)) => {
            let a_sorted = get_sorted_use_tree(sort_map, a, a_span);
            let b_sorted = get_sorted_use_tree(sort_map, b, b_span);
            cmp_iter_by(a_sorted, b_sorted, |&ai, &bi| {
                use_tree_order(&a[ai].0, &b[bi].0, sort_map)
            })
        }
        _ => unreachable!(),
    })
}

enum Element<'a> {
    Segment(Symbol),
    Glob,
    Nested(&'a [(ast::UseTree, ast::NodeId)], Span),
}

fn iter_elements(use_tree: &ast::UseTree) -> impl Iterator<Item = Element<'_>> {
    let prefix = use_tree.prefix.segments.iter().map(|s| {
        Element::Segment(s.ident.name)
    });
    prefix.chain(match use_tree.kind {
        ast::UseTreeKind::Simple(_) => None,
        ast::UseTreeKind::Glob => Some(Element::Glob),
        ast::UseTreeKind::Nested { ref items, span } => Some(Element::Nested(items, span)),
    })
}
