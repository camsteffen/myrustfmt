// test-kind: before-after

fn test() {
    // single item
    [asdfasdf({
        let x;
    })
    .aaaa
    .aaaa];
    // multiple items
    [first, asdfasdf({
        let x;
    })
    .aaaa
    .aaaa];
}

// :after:

fn test() {
    // single item
    [
        asdfasdf({
            let x;
        })
        .aaaa
        .aaaa,
    ];
    // multiple items
    [
        first,
        asdfasdf({
            let x;
        })
        .aaaa
        .aaaa,
    ];
}
