// test-kind: breakpoint

fn test() {
    aaaaaa
        .bbbb::<AAA, BBB>(aaaa)?;
}

// :after:

fn test() {
    aaaaaa.bbbb::<AAA, BBB>(
        aaaa,
    )?;
}
