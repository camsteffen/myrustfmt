// test-kind: before-after

fn test() {
    A { bbbbbbbbbbb: ccccc };
    A { bbbbbbbbbbb: cccccc };
    A { bbbbbbb: cccc, ..x };
    A { bbbbbbbb: cccc, ..x };
}

// :after:

fn test() {
    A { bbbbbbbbbbb: ccccc };
    A {
        bbbbbbbbbbb: cccccc,
    };
    A { bbbbbbb: cccc, ..x };
    A {
        bbbbbbbb: cccc,
        ..x
    };
}
