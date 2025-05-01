// test-kind: before-after

fn test() {
    foo::<>;
}

// :after:

fn test() {
    foo;
}
