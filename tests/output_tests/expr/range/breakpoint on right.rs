// test-kind: breakpoint

fn test() {
    aaaaa..(bb)
}

// :after:

fn test() {
    aaaaa..(
        bb
    )
}
