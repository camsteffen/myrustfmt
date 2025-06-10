// test-kind: breakpoint

fn test() {
    aaaa(
        BBBBB { cccccccc },
    );
}

// :after:

fn test() {
    aaaa(BBBBB {
        cccccccc,
    });
}
