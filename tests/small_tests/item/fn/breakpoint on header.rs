// test-kind: breakpoint

fn test(arg: u32) {
    x;
}

// :after:

fn test(
    arg: u32,
) {
    x;
}
