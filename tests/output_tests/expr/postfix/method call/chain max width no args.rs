// test-kind: before-after

fn test() {
    // exactly max width
    aaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
    // one over max width
    aaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();

    // one under distance to enforce max width
    aaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
    // at distance to enforce max width
    aaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
}

// :after:

fn test() {
    // exactly max width
    aaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
    // one over max width
    aaaaaaaaaaaaaaaaaaaaaa
        .bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();

    // one under distance to enforce max width
    aaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
    // at distance to enforce max width
    aaaaaaaaaaaaaaa
        .bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb();
}
