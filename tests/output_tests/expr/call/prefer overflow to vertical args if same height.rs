// test-kind: before-after
// max-width: 27

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
