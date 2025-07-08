// test-kind: breakpoint

fn test() {
    foo::<AAA, BBB>();
}

// :after:

fn test() {
    foo::<
        AAA,
        BBB,
    >();
}
