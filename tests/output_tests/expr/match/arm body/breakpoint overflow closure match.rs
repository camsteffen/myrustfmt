// test-kind: breakpoint
// note: Always add a block to arm body if it yields a longer first line.

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
