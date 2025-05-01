use std::cmp::Ordering;

pub fn cmp_by_key<T, U>(a: T, b: T, f: impl Fn(T) -> U) -> Ordering
where
    U: Ord,
{
    f(a).cmp(&f(b))
}

pub fn cmp_iter_by<T>(
    a: impl IntoIterator<Item = T>,
    b: impl IntoIterator<Item = T>,
    cmp: impl Fn(T, T) -> Ordering,
) -> Ordering {
    let mut a_iter = a.into_iter();
    let mut b_iter = b.into_iter();
    loop {
        match (a_iter.next(), b_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => match cmp(a, b) {
                Ordering::Equal => {}
                ord => return ord,
            },
        }
    }
}

pub fn cmp_iter_by_key<T, U>(
    a: impl IntoIterator<Item = T>,
    b: impl IntoIterator<Item = T>,
    f: impl Fn(T) -> U,
) -> Ordering
where
    U: Ord,
{
    cmp_iter_by(a, b, |a, b| cmp_by_key(a, b, &f))
}
