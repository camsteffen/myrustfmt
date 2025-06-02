// test-kind: breakpoint

fn test() {
    for (foo, bar) in baz {
        x;
    }
}

// :after:

fn test() {
    for (foo, bar)
        in baz
    {
        x;
    }
}
