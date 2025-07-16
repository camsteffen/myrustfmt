// test-kind: breakpoint

fn test() {
    let _ = (aaaaaaa + bbbbb)(
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
    );
}

// :after:

fn test() {
    let _ = (
        aaaaaaa + bbbbb
    )(
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
    );
}
