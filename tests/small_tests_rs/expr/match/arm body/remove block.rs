// test-kind: before-after

fn test() {
    match x {
        X => {
            x
        }
    }
}

// :after:

fn test() {
    match x {
        X => x,
    }
}
