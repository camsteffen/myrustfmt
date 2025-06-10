// test-kind: breakpoint

fn test() {
    one.two.three.four(a, || {
        x;
    });
}

// :after:

fn test() {
    one.two
        .three
        .four(a, || {
            x;
        });
}
