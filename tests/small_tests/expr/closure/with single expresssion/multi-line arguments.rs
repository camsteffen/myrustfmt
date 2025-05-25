// test-kind: before-after

fn test() {
    |
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccccccccccccccccccccccccccccccccccccccccccccc,
    | foo;
    |
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccccccccccccccccccccccccccccccccccccccccccccc,
    | match x {
        _ => {}
    };
}

// :after:

fn test() {
    |
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccccccccccccccccccccccccccccccccccccccccccccc,
    | {
        foo
    };
    |
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccccccccccccccccccccccccccccccccccccccccccccc,
    | {
        match x {
            _ => {}
        }
    };
}
