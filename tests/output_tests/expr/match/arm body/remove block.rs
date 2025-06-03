// test-kind: before-after

fn test() {
    match x {
        X => {
            x
        }
        Y => {
            y
        },
    }
}

// :after:

fn test() {
    match x {
        X => x,
        Y => y,
    }
}
