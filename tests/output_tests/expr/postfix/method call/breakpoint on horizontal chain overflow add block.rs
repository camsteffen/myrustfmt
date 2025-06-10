// test-kind: breakpoint

fn test() {
    foooo.bar(x, || match y {
        _ => {}
    });
}

// :after:

fn test() {
    foooo.bar(x, || {
        match y {
            _ => {}
        }
    });
}
