// test-kind: breakpoint

fn test() {
    if condition1 && condition2
    {
        a;
    } else {
        a;
    }
}

// :after:

fn test() {
    if condition1
        && condition2
    {
        a;
    } else {
        a;
    }
}
