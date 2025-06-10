// test-kind: breakpoint

fn test() {
    one.two.three.four.five.six(a, b);
}

// :after:

fn test() {
    one.two.three.four.five.six(
        a,
        b,
    );
}
