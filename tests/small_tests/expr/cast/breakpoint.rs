// test-kind: breakpoint

fn test() {
    aaaaaa as usize;
}

// :after:

fn test() {
    aaaaaa
        as usize;
}
