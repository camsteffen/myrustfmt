// test-kind: before-after

fn test() {
    matches![a, b];
}

// :after:

fn test() {
    matches!(a, b);
}
