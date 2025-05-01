// test-kind: breakpoint

fn test() {
    || aaaaaaaa + aaaaaaaa;
}

// :after:

fn test() {
    || {
        aaaaaaaa
            + aaaaaaaa
    };
}
