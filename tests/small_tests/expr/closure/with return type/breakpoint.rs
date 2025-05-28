// test-kind: breakpoint

fn test() {
    |arg| -> Out {
        x;
    }
}

// :after:

fn test() {
    |
        arg,
    | -> Out {
        x;
    }
}
