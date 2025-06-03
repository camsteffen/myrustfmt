// test-kind: before-after

fn test() {
    [
        aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, a, aaaaaaaaaa,
    ];
    [
        aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, a, aaaaaaaaaaa,
    ];
}

// :after:

fn test() {
    [
        aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, a, aaaaaaaaaa,
    ];
    [
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        a,
        aaaaaaaaaaa,
    ];
}
