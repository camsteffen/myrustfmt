// test-kind: breakpoint

fn test() {
    for (foo, bar)
        in [
            aaaa,
            bbbb,
        ]
    {
        x;
    }
}

// :after:

fn test() {
    for (
        foo,
        bar,
    ) in [
        aaaa,
        bbbb,
    ] {
        x;
    }
}
