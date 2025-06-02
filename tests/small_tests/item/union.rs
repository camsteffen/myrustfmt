// test-kind: no-change

union X<T>
where
    T: Copy,
{
    x: u32,
}
