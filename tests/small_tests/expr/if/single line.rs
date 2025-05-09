// test-kind: before-after

fn test() {
    // semicolon in first block
    if x { y; } else { z }
    // semicolon in else block
    if x { y } else { z; }
    // else if
    if a { b } else if c { d } else { e }
}

// :after:

fn test() {
    // semicolon in first block
    if x {
        y;
    } else {
        z
    }
    // semicolon in else block
    if x {
        y
    } else {
        z;
    }
    // else if
    if a {
        b
    } else if c {
        d
    } else {
        e
    }
}
