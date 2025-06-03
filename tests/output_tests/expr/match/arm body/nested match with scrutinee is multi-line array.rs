// test-kind: before-after

fn test() {
    match x {
        AAA => match [
            EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE,
            EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE,
        ] {
            _ => {}
        },
    }
}

// :after:

fn test() {
    match x {
        AAA => {
            match [
                EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE,
                EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE,
            ] {
                _ => {}
            }
        }
    }
}
