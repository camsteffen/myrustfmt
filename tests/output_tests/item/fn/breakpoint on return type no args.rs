// test-kind: breakpoint

fn test() -> Option<Foo> {
    x;
}

// :after:

fn test()
    -> Option<Foo>
{
    x;
}
