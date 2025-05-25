// test-kind: before-after

fn test() {
    |arg| || x;
    |arg| || {
        let x;
    };
}

// :after:

fn test() {
    |arg| || x;
    |arg| {
        || {
            let x;
        }
    };
}
