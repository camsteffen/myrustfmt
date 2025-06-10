// test-kind: before-after
// max-width: 33

fn test() {
    rooty
        .aaaa
        .bbbb(a, b, c, || xyz);
}

// :after:

fn test() {
    rooty.aaaa.bbbb(a, b, c, || {
        xyz
    });
}
