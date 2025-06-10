// test-kind: breakpoint

fn test() {
    one.two.three.four.five(a, b);
}

// :after:

fn test() {
    one.two
        .three
        .four
        .five(a, b);
}
