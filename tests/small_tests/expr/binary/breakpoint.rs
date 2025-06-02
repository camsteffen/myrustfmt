// test-kind: breakpoint

fn test() {
    aaaaaaa + bbbbbbb;
}

// :after:

fn test() {
    aaaaaaa
        + bbbbbbb;
}
