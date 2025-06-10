// test-kind: breakpoint

fn test() {
    fooooooooooooo.bbbbbbbbbbbbbbbbbbbb.cccccccc.dddddddd(
        || match x {
            _ => {}
        },
    );
}

// :after:

fn test() {
    fooooooooooooo
        .bbbbbbbbbbbbbbbbbbbb
        .cccccccc
        .dddddddd(|| match x {
            _ => {}
        });
}
