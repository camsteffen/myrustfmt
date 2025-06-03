// test-kind: before-after

fn test() {
    [a, #[aa] b]
}

// :after:

fn test() {
    [
        a,
        #[aa]
        b,
    ]
}
