// test-kind: before-after

fn test() {
    // exactly max width
    aaaaaaaaaaa.bbbbbbbbbb.ccccccccccccccccccccccccccccccccccc();
    // one over max width
    aaaaaaaaaaa.bbbbbbbbbb.cccccccccccccccccccccccccccccccccccc();

    // over max width only one wrappable item
    aaaaaaaaaaaaaaaaaaaaaa.ccccccccccccccccccccccccccccccccccccccc();
    a.b.cccccccccccccccccc.ddddddddddddddddddddddddddddddddddddddd();
}

// :after:

fn test() {
    // exactly max width
    aaaaaaaaaaa.bbbbbbbbbb.ccccccccccccccccccccccccccccccccccc();
    // one over max width
    aaaaaaaaaaa
        .bbbbbbbbbb
        .cccccccccccccccccccccccccccccccccccc();

    // over max width only one wrappable item
    aaaaaaaaaaaaaaaaaaaaaa.ccccccccccccccccccccccccccccccccccccccc();
    a.b.cccccccccccccccccc.ddddddddddddddddddddddddddddddddddddddd();
}
