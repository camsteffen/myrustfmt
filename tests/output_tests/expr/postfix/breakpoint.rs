// test-kind: breakpoint

fn test() {
    aaaaaaaaa.bbb;
}

// :after:

fn test() {
    aaaaaaaaa
        .bbb;
}
