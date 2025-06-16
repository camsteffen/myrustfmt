// test-kind: before-after
// max-width: 31

fn test() {
    aaaa(BBBBB {
        ccccc,
        dddd,
    });
}

// :after:

fn test() {
    aaaa(
        BBBBB { ccccc, dddd },
    );
}
