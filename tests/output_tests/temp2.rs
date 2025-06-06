// test-kind: breakpoint

fn test() {
    || {
        let aaaaaaaaaa = self.bbbbbbb(cccc, ddddddd(|| {
            e;
        }));
    };
}

// :after:

fn test() {
    || {
        let aaaaaaaaaa = self.bbbbbbb(
            cccc,
            ddddddd(|| {
                e;
            }),
        );
    };
}
