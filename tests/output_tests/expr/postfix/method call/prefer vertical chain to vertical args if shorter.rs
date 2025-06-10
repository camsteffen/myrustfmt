// test-kind: breakpoint

fn test() {
    one.two.three.four(a, b, c, d, e, f);
}

// :after:

fn test() {
    one.two
        .three
        .four(a, b, c, d, e, f);
}
