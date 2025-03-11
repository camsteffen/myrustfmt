// test-kind: breakpoint

fn test() {
    [
        aaaaaaaa,
        (aaaaaaaaaaa, aaaaaaaaaaa),
    ];
}

// :after:

fn test() {
    [
        aaaaaaaa,
        (
            aaaaaaaaaaa,
            aaaaaaaaaaa,
        ),
    ];
}
