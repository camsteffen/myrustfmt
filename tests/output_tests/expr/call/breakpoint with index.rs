// test-kind: breakpoint

fn test() {
    call(fooo[1]);
}

// :after:

fn test() {
    call(
        fooo[1],
    );
}
