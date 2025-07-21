// test-kind: breakpoint

impl<AAAAAA, BBBBBB>
    XXXXX<
        AAAAAA,
        BBBBBB,
    >
{
    fn f() {}
}


// :after:

impl<
    AAAAAA,
    BBBBBB,
> XXXXX<
    AAAAAA,
    BBBBBB,
> {
    fn f() {}
}
