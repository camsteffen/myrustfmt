// test-kind: before-after

fn test() {
    (  /* comment */  x );
}

// :after:

fn test() {
    ( /* comment */ x);
}
