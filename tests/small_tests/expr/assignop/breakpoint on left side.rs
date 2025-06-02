// test-kind: breakpoint

fn test() {
    foo(aaaaaaaa) += x;
}

// :after:

fn test() {
    foo(
        aaaaaaaa,
    ) += x;
}
