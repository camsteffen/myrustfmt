// test-kind: before-after

fn test() {
    match x {
        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA => foo(
            aaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbb,
            || {
                x;
            },
        ),
    }
}

// :after:

fn test() {
    match x {
        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA => {
            foo(aaaaaaaaaaaa, bbbbbbbbbbbbbbbbbb, || {
                x;
            })
        }
    }
}
