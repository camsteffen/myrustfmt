// test-kind: breakpoint

fn test() {
    aaaaa.bbb.ccc()
}

// :after:

fn test() {
    aaaaa
        .bbb
        .ccc()
}
