// test-kind: breakpoint

fn test() {
    [aaaaaa, aaaaaa]?;
}

// :after:

fn test() {
    [
        aaaaaa,
        aaaaaa,
    ]?;
}
