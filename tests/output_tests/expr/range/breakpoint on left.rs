// test-kind: breakpoint

fn test() {
    let _ = (aaaaa)..(
        bbbbb
    );
}

// :after:

fn test() {
    let _ = (
        aaaaa
    )..(bbbbb);
}
