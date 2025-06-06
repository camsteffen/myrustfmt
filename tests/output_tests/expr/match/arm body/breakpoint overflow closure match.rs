// test-kind: breakpoint

fn test() {
    match x {
        PPPPPPPPPP => foo(aaaaaaa, bbbbbbbbbb, || match x {
            _ => {}
        }),
    }
}

// :after:

fn test() {
    match x {
        PPPPPPPPPP => {
            foo(aaaaaaa, bbbbbbbbbb, || match x {
                _ => {}
            })
        }
    }
}
