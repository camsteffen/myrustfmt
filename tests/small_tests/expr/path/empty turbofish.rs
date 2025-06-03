// test-kind: before-after

fn test() {
    foo::<>;
    foo::</*heh*/>;
}

// :after:

fn test() {
    foo;
    foo/*heh*/;
}
