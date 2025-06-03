// test-kind: before-after

fn test() {
    match x {
        x => aaaaaaa({
            let x;
        })
        .aaaaaaa({
            let x;
        }),
    }
}

// :after:

fn test() {
    match x {
        x => {
            aaaaaaa({
                let x;
            })
            .aaaaaaa({
                let x;
            })
        }
    }
}
