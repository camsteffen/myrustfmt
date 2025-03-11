// test-kind: breakpoint

fn test() {
    aaaa.bbbbbb(arg)?;
}

// :after:

fn test() {
    aaaa.bbbbbb(
        arg,
    )?;
}
