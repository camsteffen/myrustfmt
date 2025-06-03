// test-kind: before-after

fn test() {
    foo(abc, bar(|| {
        hi;
    }))
}

// :after:

fn test() {
    foo(
        abc,
        bar(|| {
            hi;
        }),
    )
}