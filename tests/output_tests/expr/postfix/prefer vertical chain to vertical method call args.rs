// test-kind: breakpoint
// note: The reduced max width makes the method call arg overflow no longer possible, so there is a
// note: choice between formatting the method call args vertically or formatting the chain
// note: vertically.

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
