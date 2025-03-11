// test-kind: before-after

fn test() {
    || {
        match x {
            _ => {}
        }
    };
}

// :after:

fn test() {
    || match x {
        _ => {}
    };
}
