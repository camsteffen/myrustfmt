// test-kind: breakpoint

fn test() {
    aaaaaaaaa.bbbb::<
        AAA,
        BBB,
    >(aaaaa)?;
}

// :after:

fn test() {
    aaaaaaaaa
        .bbbb::<
            AAA,
            BBB,
        >(aaaaa)?;
}
