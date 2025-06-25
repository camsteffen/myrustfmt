// test-kind: breakpoint

fn test() {
    matches!(aaaaa, BBBBBBB);
}

// :after:

fn test() {
    matches!(
        aaaaa,
        BBBBBBB,
    );
}
