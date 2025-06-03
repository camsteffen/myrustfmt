// test-kind: before-after

fn test() {
    |arg| if x {
        y;
    };
    |arg| if x {
        yyyyyyyyyyyyyyyy
    } else {
        zzzzzzzzzzzzzzzz
    };
    |arg| for x in y {
        z;
    };
    |arg| loop {
        x;
    };
    |arg| while x {
        y;
    };
    |arg| {
        match x {
            _ => {}
        }
    };
    |arg| Struct { aaaaaaaaaaaaa, bbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccc };
    |arg| call(aaaaaaaaaaaaa, bbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccccccccccccccccccccccc);
    |arg| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        .bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        .cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc;
    |arg| aaaaaaaaaaaaaa({
        x;
    })
    .bbbbbbbb;
}

// :after:

fn test() {
    |arg| {
        if x {
            y;
        }
    };
    |arg| {
        if x {
            yyyyyyyyyyyyyyyy
        } else {
            zzzzzzzzzzzzzzzz
        }
    };
    |arg| {
        for x in y {
            z;
        }
    };
    |arg| {
        loop {
            x;
        }
    };
    |arg| {
        while x {
            y;
        }
    };
    |arg| match x {
        _ => {}
    };
    |arg| {
        Struct {
            aaaaaaaaaaaaa,
            bbbbbbbbbbbbbb,
            cccccccccccccccccccccccccccccccc,
        }
    };
    |arg| {
        call(
            aaaaaaaaaaaaa,
            bbbbbbbbbbbbbb,
            cccccccccccccccccccccccccccccccccccccccccccccccccccc,
        )
    };
    |arg| {
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
            .bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
            .cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
    };
    |arg| {
        aaaaaaaaaaaaaa({
            x;
        })
        .bbbbbbbb
    };
}
