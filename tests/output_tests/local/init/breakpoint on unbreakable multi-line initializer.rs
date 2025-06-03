// test-kind: breakpoint

fn test() {
    let x = Struct {
        y,
    };
}

// :after:

fn test() {
    let x =
        Struct {
            y,
        };
}
