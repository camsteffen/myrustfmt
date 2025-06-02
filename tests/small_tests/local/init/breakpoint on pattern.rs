// test-kind: breakpoint

fn test() {
    let (aaa, bbb) =
        [xxxx];
}

// :after:

fn test() {
    let (
        aaa,
        bbb,
    ) = [xxxx];
}
