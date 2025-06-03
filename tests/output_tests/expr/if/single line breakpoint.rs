// test-kind: breakpoint

fn test() {
    if x { y } else { z };
}

// :after:

fn test() {
    if x {
        y
    } else {
        z
    };
}
