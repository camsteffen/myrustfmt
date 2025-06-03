// test-kind: no-change

fn test<'a: 'b, const X: u32 = 1, T: Trait>() {}
