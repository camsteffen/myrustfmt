// test-kind: before-after

fn test() {
    match x {
        | () => {}
    }
}

// :after:

fn test() {
    match x {
        () => {}
    }
}
