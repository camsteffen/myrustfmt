// test-kind: breakpoint

fn test() {
    x = [
        aaaa, aaaa, aaaa, X { x },
    ];
}

// :after:

fn test() {
    // this is currently failing because the list builder requires VerticalList
    // but expr_list_item sets shape to BlockLike
    x = [aaaa, aaaa, aaaa, X {
        x
    }];
}
