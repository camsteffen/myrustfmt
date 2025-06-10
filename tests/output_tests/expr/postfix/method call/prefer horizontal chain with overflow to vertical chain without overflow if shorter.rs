// test-kind: before-after
// max-width: 40

fn test() {
    rooty
        .aaaa
        .bbbb
        .cccc(a, b, c, || xyz);
}

// :after:

fn test() {
    rooty.aaaa.bbbb.cccc(a, b, c, || {
        xyz
    });
}
