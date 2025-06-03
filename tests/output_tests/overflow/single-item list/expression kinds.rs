// test-kind: before-after

fn test() {
    [if x {
        y;
    }];
    [for x in y {
        z;
    }];
    [loop {
        x;
    }];
    [match x {
        _ => {}
    }];
    [while x {
        y;
    }];
}

// :after:

fn test() {
    [
        if x {
            y;
        },
    ];
    [
        for x in y {
            z;
        },
    ];
    [
        loop {
            x;
        },
    ];
    [
        match x {
            _ => {}
        },
    ];
    [
        while x {
            y;
        },
    ];
}
