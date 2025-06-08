use crate::util::cmp::{cmp_by_key, cmp_iter_by};
use std::cmp::Ordering;

// "version sort" is specified here: https://doc.rust-lang.org/nightly/style-guide/#sorting
pub fn version_sort(a: &str, b: &str) -> Ordering {
    enum Chunk<'a> {
        Number(&'a str),
        NonNumber(&'a str),
    }
    fn chunks(s: &str) -> impl Iterator<Item = Chunk> {
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
            (Chunk::Number(a), Chunk::Number(b)) => cmp_by_key(a, b, |s| {
                s.parse::<u32>().expect("numeric chunk should be a valid u32")
            }),
            (Chunk::NonNumber(str_a), Chunk::NonNumber(str_b)) => {
                cmp_iter_by(str_a.bytes(), str_b.bytes(), |byte_a, byte_b| {
                    cmp_by_key(byte_a, byte_b, |b| b != b'_').then_with(|| byte_a.cmp(&byte_b))
                })
            }
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
    assert_eq!(version_sort("B", "a"), Ordering::Less);
    assert_eq!(version_sort("a11", "a012"), Ordering::Less);
    assert_eq!(version_sort("001", "01"), Ordering::Less);
}
