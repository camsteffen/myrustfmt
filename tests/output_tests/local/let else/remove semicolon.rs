// test-kind: before-after

fn test() {
    let Some(_) = x else {
        return y;
    };
}

// :after:

fn test() {
    let Some(_) = x else { return y };
}
