// test-kind: no-change

fn test() -> impl for<'a, 'b> X<F = U> + use<'a, 'b, T> {}

fn trait_modifiers() -> impl !Sized + ?Sized + const X + ~const async Y {}
