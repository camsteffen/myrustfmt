// test-kind: breakpoint

fn test() {
    let aaaaa: (bbbbb,);
}

// :after:

fn test() {
    let aaaaa: (
        bbbbb,
    );
}
