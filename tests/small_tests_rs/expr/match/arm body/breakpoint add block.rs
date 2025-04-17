// test-kind: breakpoint

fn test() {
    match x {
        _ => [asdfasdf],
    }
}

// :after:

fn test() {
    match x {
        _ => {
            [asdfasdf]
        }
    }
}
