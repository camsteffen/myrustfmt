// test-kind: breakpoint

fn test() {
    matches!(
        aaaaa,
        BBBBBBB if ccccc,
    );
}

// :after:

fn test() {
    matches!(
        aaaaa,
        BBBBBBB
            if ccccc,
    );
}
