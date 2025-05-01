// test-kind: breakpoint

fn test() {
    // wrap bbb instead of overflow - it would be the same height
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbb(ccccccccccccccccccccccc));
}

// :after:

fn test() {
    // wrap bbb instead of overflow - it would be the same height
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(
        bbbbbbbbbbbbbbbbbbbbbbbb(ccccccccccccccccccccccc),
    );
}
