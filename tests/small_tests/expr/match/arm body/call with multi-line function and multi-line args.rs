// test-kind: before-after

fn test() {
    match x {
        _ => (
            aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                .aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                .aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        )({
            let x;
        }),
    }
}

// :after:

fn test() {
    match x {
        _ => {
            (
                aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                    .aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                    .aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
            )({
                let x;
            })
        }
    }
}
