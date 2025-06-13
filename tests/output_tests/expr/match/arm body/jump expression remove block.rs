// test-kind: before-after

fn test() {
    match x {
        _ => {
            return x;
        }
    }
}

// :after:

fn test() {
    match x {
        _ => return x,
    }
}
