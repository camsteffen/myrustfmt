// test-kind: breakpoint

fn test() {
    foooo(|arg, arg| {
        expr
    });
}

// :after:

fn test() {
    foooo(
        |arg, arg| {
            expr
        },
    );
}
