// test-kind: before-after

fn test() {
    match x {
        pppppppppppppppppppppppppppppppppppp => |
            arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
            arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
            arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
            arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
        | x,
    }
}

// :after:

fn test() {
    match x {
        pppppppppppppppppppppppppppppppppppp => {
            |
                arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
                arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
                arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
                arrrrrrrrrrrrrrrrrrrrrrrrrrrrg,
            | {
                x
            }
        }
    }
}
