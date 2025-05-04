// test-kind: breakpoint

fn test() {
    fooooooooooo(|| {
        x
    })
}

// :after:

fn test() {
    fooooooooooo(
        || x,
    )
}
