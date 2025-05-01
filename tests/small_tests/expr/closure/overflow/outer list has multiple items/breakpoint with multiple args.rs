// test-kind: breakpoint

fn test() {
    foo(first_arg, |arg, arg| {
        result
    });
}

// :after:

fn test() {
    foo(
        first_arg,
        |arg, arg| result,
    );
}
