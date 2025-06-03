// test-kind: breakpoint

fn test() {
    [aaaaaaaaaaaa + aaaaaaa; 42];
}

// :after:

fn test() {
    [aaaaaaaaaaaa
        + aaaaaaa; 42];
}
