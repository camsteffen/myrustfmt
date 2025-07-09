// test-kind: breakpoint

fn test() {
    aaaaaa.bbbbbbb.cccc::<AAA, BBB>(
        aaaaa,
        bbbbb,
    )?;
}

// :after:

fn test() {
    aaaaaa
        .bbbbbbb
        .cccc::<AAA, BBB>(
            aaaaa,
            bbbbb,
        )?;
}
