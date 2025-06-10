// test-kind: before-after
// max-width: 45

fn test() {
    rooty.aaaa.bbbb.cccccccc(Thing {
        a,
        b,
        c,
    });
}

// :after:

fn test() {
    rooty
        .aaaa
        .bbbb
        .cccccccc(Thing { a, b, c });
}
