// test-kind: before-after

enum X {
    A { aaaaaaaaaa: BBBBBBBBB, ccccccc: DDD },
    B { aaaaaaaaaa: BBBBBBBBB, ccccccc: DDDD },
}

// :after:

enum X {
    A { aaaaaaaaaa: BBBBBBBBB, ccccccc: DDD },
    B {
        aaaaaaaaaa: BBBBBBBBB,
        ccccccc: DDDD,
    },
}
