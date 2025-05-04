// test-kind: breakpoint

fn test() {
    let xxxxxxxxxxxxx = aaaaaaaaaa else {
        return x;
    };
}

// :after:

fn test() {
    let xxxxxxxxxxxxx = aaaaaaaaaa
    else {
        return x;
    };
}
