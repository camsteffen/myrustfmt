// test-kind: before-after

fn test() {
    if cond {  // super helpful comment
        booya();
    }
}

// :after:

fn test() {
    if cond { // super helpful comment
        booya();
    }
}
