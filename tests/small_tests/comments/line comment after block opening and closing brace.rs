// test-kind: before-after

fn test() {
    {    // open
        let x;
    }    // close
}

// :after:

fn test() {
    { // open
        let x;
    } // close
}
