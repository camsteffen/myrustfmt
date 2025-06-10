// test-kind: breakpoint

fn test() {
    fooooooooooooo.bbbbbbbbbbbbbbbbbbbb.cccccccccccccccc(
        || match x {
            _ => {}
        },
    );
}

// :after:

fn test() {
    fooooooooooooo
        .bbbbbbbbbbbbbbbbbbbb
        .cccccccccccccccc(|| match x {
            _ => {}
        });
}
