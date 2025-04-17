// test-kind: before-after

fn test() {
    match x {
        _ => if y {
            z;
        }
    }
}

// :after:

fn test() {
    match x {
        _ => {
            if y {
                z;
            }
        }
    }
}
