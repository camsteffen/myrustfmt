// test-kind: before-after
// max-width: 22

fn test() -> Option<Foo> {
    x;
}

// :after:

fn test()
    -> Option<Foo>
{
    x;
}
