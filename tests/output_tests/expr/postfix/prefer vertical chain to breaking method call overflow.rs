// test-kind: breakpoint

fn test() {
    oneone.two.three(Foo {
        foooooey,
        gooooooey,
    })
}

// :after:

fn test() {
    oneone
        .two
        .three(Foo {
            foooooey,
            gooooooey,
        })
}
