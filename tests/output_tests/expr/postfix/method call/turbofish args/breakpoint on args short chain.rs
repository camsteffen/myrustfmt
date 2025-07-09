// test-kind: breakpoint

fn test() {
    aaaa.bbbb::<AAA, BBB>(aaaa)?;
}

// :after:

fn test() {
    aaaa.bbbb::<AAA, BBB>(
        aaaa,
    )?;
}
