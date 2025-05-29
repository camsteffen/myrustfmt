// test-kind: breakpoint

fn test() {
    return || value;
}

// :after:

fn test() {
    return || {
        value
    };
}
