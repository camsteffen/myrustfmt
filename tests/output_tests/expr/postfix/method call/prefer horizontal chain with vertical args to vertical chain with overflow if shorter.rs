// test-kind: breakpoint

fn test() {
    one.two.three.four.five.six(a, || {
        x;
    });
}

// :after:

fn test() {
    one.two.three.four.five.six(
        a,
        || {
            x;
        },
    );
}
