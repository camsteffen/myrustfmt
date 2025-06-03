// test-kind: before-after

fn test() {
    [a, if x {
        y;
    }];
    [a, for x in y {
        z;
    }];
    [a, loop {
        x;
    }];
    [a, match x {
        _ => {}
    }];
    [a, while x {
        y;
    }];
}

// :after:

fn test() {
    [
        a,
        if x {
            y;
        },
    ];
    [
        a,
        for x in y {
            z;
        },
    ];
    [
        a,
        loop {
            x;
        },
    ];
    [
        a,
        match x {
            _ => {}
        },
    ];
    [
        a,
        while x {
            y;
        },
    ];
}
