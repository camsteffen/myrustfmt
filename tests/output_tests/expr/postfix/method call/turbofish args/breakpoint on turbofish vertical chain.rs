// test-kind: breakpoint

fn test() {
    aaaaaa
        .bbbbbbbbbb
        .cccc::<AAA, BBB>(
            aaaaaaaa,
            aaaaaaaa,
        )?;
}

// :after:

fn test() {
    aaaaaa
        .bbbbbbbbbb
        .cccc::<
            AAA,
            BBB,
        >(
            aaaaaaaa,
            aaaaaaaa,
        )?;
}
