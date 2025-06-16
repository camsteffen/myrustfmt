// test-kind: before-after

fn test() {
    A { bbbbbbbbbbb: ccccc };
    A { bbbbbbbbbbb: cccccc };
}

// :after:

fn test() {
    A { bbbbbbbbbbb: ccccc };
    A {
        bbbbbbbbbbb: cccccc,
    };
}
