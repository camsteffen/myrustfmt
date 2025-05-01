// test-kind: before-after

fn test() {
    || foo(|| {
        x;
    });
}

// :after:

fn test() {
    || {
        foo(|| {
            x;
        })
    };
}
