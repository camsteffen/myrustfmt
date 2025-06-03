// test-kind: before-after

fn test() {
    match x {
        _ => {
            let x;
        },
    }
}

// :after:

fn test() {
    match x {
        _ => {
            let x;
        }
    }
}
