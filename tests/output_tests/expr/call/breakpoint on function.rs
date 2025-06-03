// test-kind: breakpoint

fn test() {
    (aaaaaaa + bbbbb)(
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
    );
}

// :after:

fn test() {
    (
        aaaaaaa
            + bbbbb
    )(
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
        aaaaaaaaaa,
    );
}
