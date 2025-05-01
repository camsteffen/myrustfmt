// test-kind: before-after

fn test() {
    || if condition {
        some_value
    } else {
        some_other_value
    };
}

// :after:

fn test() {
    || {
        if condition {
            some_value
        } else {
            some_other_value
        }
    };
}
