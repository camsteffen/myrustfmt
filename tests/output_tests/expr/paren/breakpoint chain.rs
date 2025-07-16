// test-kind: breakpoint

fn test() {
    break (aaaaaaa + bbbbb);
}

// :after:

fn test() {
    break (
        aaaaaaa + bbbbb
    );
}
