// test-kind: no-change

fn test() {
    // barely don't wrap
    aaa + b
        + cccccccccccccccccccccccccccccccccccccccc
        + dddddddddddddddddddddddddddddddddddddddd
        + eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee;

    // barely wrap
    aaaa
        + b
        + cccccccccccccccccccccccccccccccccccccccc
        + dddddddddddddddddddddddddddddddddddddddd
        + eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee;

    // don't wrap after multi-line item
    [
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
    ] + b
        + c;

    // force wrap with line comment
    a // hehe
        + b;
}
