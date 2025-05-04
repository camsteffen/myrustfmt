// test-kind: before-after

fn test() {
    Self::new(Rc::new(source.into()), Constraints::default(), Rc::new(
        ErrorEmitter::new(None),
    ))
}

// :after:

fn test() {
    Self::new(
        Rc::new(source.into()),
        Constraints::default(),
        Rc::new(ErrorEmitter::new(None)),
    )
}
