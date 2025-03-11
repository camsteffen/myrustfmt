// test-kind: before-after

fn test() {
    // at max width of 60 chars
    [aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaaaa];
    // one over max width - wrap-to-fit
    [aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaaaaa];
    // one over max width - separate lines
    [aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaaaaaaaaaaa];
}

// :after:

fn test() {
    // at max width of 60 chars
    [aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaaaa];
    // one over max width - wrap-to-fit
    [
        aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaa, aaaaaaa,
    ];
    // one over max width - separate lines
    [
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaa,
        aaaaaaaaaaaaa,
    ];
}
