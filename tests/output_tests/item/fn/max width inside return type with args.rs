// test-kind: before-after
// max-width: 30

fn test(arg: Type) -> Option<Foo> {
    x;
}

// :after:

fn test(
    arg: Type,
) -> Option<Foo> {
    x;
}
