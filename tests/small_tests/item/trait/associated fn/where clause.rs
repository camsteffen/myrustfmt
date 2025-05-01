// test-kind: no-change

trait X {
    fn test<T, U>()
    where
        T: Copy,
        U: Copy;

    fn test<T, U>()
    where
        T: Copy,
    {}
}
