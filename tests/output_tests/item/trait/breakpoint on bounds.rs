// test-kind: breakpoint

trait Trait<T>: Foo
{}

// :after:

trait Trait<T>:
    Foo
{}
