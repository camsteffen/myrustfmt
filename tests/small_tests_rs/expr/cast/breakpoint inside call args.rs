// test-kind: breakpoint

fn test() {
    fooooo(
        aaaaaa as usize,
    );
}

// :after:

fn test() {
    fooooo(
        aaaaaa
            as usize,
    );
}
