use crate::util::cmp::{cmp_by_key, cmp_iter_by};
use rustc_ast::ast;
use rustc_span::Symbol;
use rustc_span::kw;
use std::cmp::Ordering;

pub fn use_tree_order(a: &ast::UseTree, b: &ast::UseTree) -> Ordering {
    cmp_iter_by(iter(a), iter(b), |element_a, element_b| {
        cmp_by_key(&element_a, &element_b, |e| match e {
            Element::Segment(_) => 0,
            Element::Glob => 1,
            Element::Nested(_) => 2,
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
            (Element::Nested(a), Element::Nested(b)) => {
                // todo cache sorting
                // todo reuse sorting between ordering and formatting
                let mut sorted_a = Vec::from_iter(a.iter().map(|(use_tree, _)| use_tree));
                let mut sorted_b = Vec::from_iter(b.iter().map(|(use_tree, _)| use_tree));
                sorted_a.sort_by(|a, b| use_tree_order(a, b));
                sorted_b.sort_by(|a, b| use_tree_order(a, b));
                cmp_iter_by(sorted_a, sorted_b, use_tree_order)
            }
            _ => unreachable!(),
        })
    })
}

enum Element<'a> {
    Segment(Symbol),
    Glob,
    Nested(&'a [(ast::UseTree, ast::NodeId)]),
}

fn iter(use_tree: &ast::UseTree) -> impl Iterator<Item = Element<'_>> {
    let prefix = use_tree
        .prefix
        .segments
        .iter()
        .map(|s| Element::Segment(s.ident.name));
    prefix.chain(std::iter::once(()).filter_map(|()| match &use_tree.kind {
        ast::UseTreeKind::Simple(_) => None,
        ast::UseTreeKind::Glob => Some(Element::Glob),
        ast::UseTreeKind::Nested { items, .. } => Some(Element::Nested(items)),
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

// "version sort" is specified here: https://doc.rust-lang.org/nightly/style-guide/#sorting
fn version_sort(a: &str, b: &str) -> Ordering {
    enum Chunk<'a> {
        Number(&'a str),
        NonNumber(&'a str),
    }
    fn chunks(s: &str) -> impl Iterator<Item = Chunk<'_>> {
        let mut remaining = s;
        std::iter::from_fn(move || {
            let mut chars = remaining.chars();
            let is_digit = chars.next()?.is_ascii_digit();
            let after_first = chars.as_str();
            let end = after_first
                .find(|c: char| c.is_ascii_digit() != is_digit)
                .unwrap_or(after_first.len());
            let len = remaining.len() - after_first.len() + end;
            let chunk_str;
            (chunk_str, remaining) = remaining.split_at(len);
            if is_digit {
                Some(Chunk::Number(chunk_str))
            } else {
                Some(Chunk::NonNumber(chunk_str))
            }
        })
    }

    cmp_iter_by(chunks(a), chunks(b), |chunk_a, chunk_b| {
        match (chunk_a, chunk_b) {
            (Chunk::Number(_), Chunk::NonNumber(_)) => Ordering::Less,
            (Chunk::NonNumber(_), Chunk::Number(_)) => Ordering::Greater,
            (Chunk::Number(a), Chunk::Number(b)) => {
                cmp_by_key(a, b, |s| s.parse::<u32>().expect("numeric chunk should be a valid u32"))
            }
            (Chunk::NonNumber(str_a), Chunk::NonNumber(str_b)) => cmp_iter_by(
                str_a.bytes(),
                str_b.bytes(),
                |byte_a, byte_b| {
                    cmp_by_key(byte_a, byte_b, |b| b != b'_').then_with(|| byte_a.cmp(&byte_b))
                },
            ),
        }
    })
    .then_with(|| {
        cmp_iter_by(chunks(a), chunks(b), |chunk_a, chunk_b| {
            match (chunk_a, chunk_b) {
                (Chunk::Number(a), Chunk::Number(b)) => {
                    cmp_by_key(a, b, |c| c.bytes().take_while(|&b| b == b'0').count())
                        .reverse()
                }
                _ => Ordering::Equal,
            }
        })
    })
}

#[test]
fn test_version_sort() {
    assert_eq!(version_sort("_", "A"), Ordering::Less);
    assert_eq!(version_sort("A", "a"), Ordering::Less);
    assert_eq!(version_sort("a11", "a012"), Ordering::Less);
    assert_eq!(version_sort("001", "01"), Ordering::Less);
}
