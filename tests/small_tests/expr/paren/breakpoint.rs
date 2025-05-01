// test-kind: breakpoint

fn test() {
    (aaaaaaa + bbbbb)
}

// :after:

fn test() {
    (
        aaaaaaa
            + bbbbb
    )
}
