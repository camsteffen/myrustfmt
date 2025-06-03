// test-kind: breakpoint

fn test() {
    let x = y;
}

// :after:

fn test() {
    let x =
        y;
}
