// test-kind: breakpoint

fn test() {
    match x {
        (aaa, bbb) => {
            x;
        }
    }
}

// :after:

fn test() {
    match x {
        (
            aaa,
            bbb,
        ) => {
            x;
        }
    }
}
