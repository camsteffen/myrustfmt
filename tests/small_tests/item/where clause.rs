// test-kind: no-change

struct X<'a, T>
where
    'a: 'b,
    T: Trait + 'a;
