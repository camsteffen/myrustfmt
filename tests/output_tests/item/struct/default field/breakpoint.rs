// test-kind: breakpoint

struct X {
    a: BBB = c,
}

// :after:

struct X {
    a: BBB
        = c,
}
