// test-kind: before-after
// max-width: 20

fn test() {
    match x {
        _ => aaaaaaaaaa,
    }
}

// :after:

fn test() {
    match x {
        _ => {
            aaaaaaaaaa
        }
    }
}
