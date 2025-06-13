// test-kind: breakpoint

fn test() {
    match x {
        _ => return x,
    }
}

// :after:

fn test() {
    match x {
        _ => {
            return x;
        }
    }
}
