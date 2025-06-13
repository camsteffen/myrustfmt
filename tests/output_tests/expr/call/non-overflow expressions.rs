// test-kind: before-after
// max-width: 30

fn test() {
    call(aaaa, X {
        aaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaa,
    });
    call(aaaa, {
        x;
    });
    call(aaaa, unsafe {
        x;
    });
    call(aaaa, [
        aaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaa,
    ]);
    call(aaaa, if x {
        y;
    });
    call(aaaa, match x {
        _ => {}
    });
}

// :after:

fn test() {
    call(
        aaaa,
        X {
            aaaaaaaaaaaaaaaa,
            aaaaaaaaaaaaaaaa,
        },
    );
    call(
        aaaa,
        {
            x;
        },
    );
    call(
        aaaa,
        unsafe {
            x;
        },
    );
    call(
        aaaa,
        [
            aaaaaaaaaaaaaaaa,
            aaaaaaaaaaaaaaaa,
        ],
    );
    call(
        aaaa,
        if x {
            y;
        },
    );
    call(
        aaaa,
        match x {
            _ => {}
        },
    );
}
