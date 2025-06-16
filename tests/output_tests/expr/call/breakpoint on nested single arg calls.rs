// test-kind: breakpoint
// note: prefer breaking outer calls

fn test() {
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbb(ccccccccccccccccccccccc));
}

// :after:

fn test() {
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbb(
        ccccccccccccccccccccccc,
    ));
}
