// test-kind: breakpoint

fn test() {
    aaaa.bbbb::<AAA, BBB>()?;
}

// :after:

fn test() {
    aaaa.bbbb::<
        AAA,
        BBB,
    >()?;
}
