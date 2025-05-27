// test-kind: breakpoint

pub fn test() {
    match x {
        Pattern if condition => {
            x;
        }
    }
}

// :after:

pub fn test() {
    match x {
        Pattern
            if condition =>
        {
            x;
        }
    }
}
