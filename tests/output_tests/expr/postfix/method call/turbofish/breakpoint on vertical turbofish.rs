// test-kind: breakpoint

fn test() {
    aaaaaa.bbbb::<
        AAA,
        BBB,
    >()?;
}

// :after:

fn test() {
    aaaaaa
        .bbbb::<
            AAA,
            BBB,
        >()?;
}
