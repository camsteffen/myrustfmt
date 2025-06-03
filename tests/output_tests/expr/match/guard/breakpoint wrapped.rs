// test-kind: breakpoint

pub fn test() {
    match x {
        Pattern
            if (aaa, bbb) =>
        {
            x;
        }
    }
}

// :after:

pub fn test() {
    match x {
        Pattern
            if (
                aaa,
                bbb,
            ) =>
        {
            x;
        }
    }
}
