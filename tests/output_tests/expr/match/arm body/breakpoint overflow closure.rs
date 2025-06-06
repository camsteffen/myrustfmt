// test-kind: breakpoint

fn test() {
    match x {
        PPPPPPPPPP => foo(aaaaaaa, bbbbbbbbbb, || {
            x;
        }),
    }
}

// :after:

fn test() {
    match x {
        PPPPPPPPPP => {
            foo(aaaaaaa, bbbbbbbbbb, || {
                x;
            })
        }
    }
}
