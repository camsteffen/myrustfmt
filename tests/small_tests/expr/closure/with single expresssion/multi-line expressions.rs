// test-kind: before-after

fn test() {
    || if x {
        y;
    };
    || if x {
        yyyyyyyyyyyyyyyy
    } else {
        zzzzzzzzzzzzzzzz
    };
    || for x in y {
        z;
    };
    || loop {
        x;
    };
    || while x {
        y;
    };
    || {
        match x {
            _ => {}
        }
    };
    || Struct { aaaaaaaaaaaaa, bbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccc };
    || call(aaaaaaaaaaaaa, bbbbbbbbbbbbbb, cccccccccccccccccccccccccccccccccccccccccccccccccccc);
}

// :after:

fn test() {
    || {
        if x {
            y;
        }
    };
    || {
        if x {
            yyyyyyyyyyyyyyyy
        } else {
            zzzzzzzzzzzzzzzz
        }
    };
    || {
        for x in y {
            z;
        }
    };
    || {
        loop {
            x;
        }
    };
    || {
        while x {
            y;
        }
    };
    || match x {
        _ => {}
    };
    || {
        Struct {
            aaaaaaaaaaaaa,
            bbbbbbbbbbbbbb,
            cccccccccccccccccccccccccccccccc,
        }
    };
    || {
        call(
            aaaaaaaaaaaaa,
            bbbbbbbbbbbbbb,
            cccccccccccccccccccccccccccccccccccccccccccccccccccc,
        )
    };
}
