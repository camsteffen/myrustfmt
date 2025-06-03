// test-kind: breakpoint

fn test() {
    match x {
        Foooooo(bar) => [asdfasdf],
    }
}

// :after:

fn test() {
    match x {
        Foooooo(bar) => {
            [asdfasdf]
        }
    }
}
