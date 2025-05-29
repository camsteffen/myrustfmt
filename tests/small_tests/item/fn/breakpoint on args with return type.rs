// test-kind: breakpoint

fn test(arg: u32) -> Option<Foo> {
    x;
}

// :after:

fn test(
    arg: u32,
) -> Option<Foo> {
    x;
}
