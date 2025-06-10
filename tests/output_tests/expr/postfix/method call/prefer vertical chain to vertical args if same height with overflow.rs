// test-kind: breakpoint

fn test() {
    one.two.three.four.five(a, || {
        x;
    });
}

// :after:

fn test() {
    one.two
        .three
        .four
        .five(a, || {
            x;
        });
}
