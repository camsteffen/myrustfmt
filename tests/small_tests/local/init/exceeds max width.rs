// test-kind: before-after
// max-width: 25

fn test() {
    let (aaaa, bbbb) = aaaaaaaaaaaaaaaaaaaa;
}

// :after:

fn test() {
    let (aaaa, bbbb) =
        aaaaaaaaaaaaaaaaaaaa;
}
