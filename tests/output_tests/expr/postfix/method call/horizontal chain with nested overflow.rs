// test-kind: no-change
// max-width: 40
// note: If you try to format the method call args horizontally, the max-width is exceeded before
// note: the closure *unless* you wrap the method call to the next line.

fn test() {
    let aaaaaaaaaa = self.bbbbbbb(
        cccc,
        dddd(|| {
            e;
        }),
    );
}
