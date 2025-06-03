// test-kind: breakpoint

fn test() {
    const X: (Type,);
}

// :after:

fn test() {
    const X: (
        Type,
    );
}
