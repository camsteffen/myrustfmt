// test-kind: breakpoint

fn test() {
    const { x };
}

// :after:

fn test() {
    const {
        x
    };
}
