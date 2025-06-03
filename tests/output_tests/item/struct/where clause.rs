// test-kind: no-change

struct Foo<X>
where
    X: Y;

struct Foo<X>
where
    X: Y,
{
    x: y,
}
