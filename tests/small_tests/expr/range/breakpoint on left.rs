// test-kind: breakpoint

fn test() {
    (aaaaa)..(
        bbbbb
    )
}

// :after:

fn test() {
    (
        aaaaa
    )..(
        bbbbb
    )
}
