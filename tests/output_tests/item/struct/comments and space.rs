// test-kind: before-after

struct X(

    /// A
    A,

    /// B
    B,

);

struct Y {

    x: X,


    y: Y,

}

// :after:

struct X(
    /// A
    A,
    /// B
    B,
);

struct Y {
    x: X,

    y: Y,
}
