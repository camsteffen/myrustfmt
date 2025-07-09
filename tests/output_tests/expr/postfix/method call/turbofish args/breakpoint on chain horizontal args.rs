// test-kind: breakpoint

fn test() {
    aaaaaa.bbbb::<AAA, BBB>(aaaaa)?;
}

// :after:

fn test() {
    aaaaaa
        .bbbb::<AAA, BBB>(aaaaa)?;
}
