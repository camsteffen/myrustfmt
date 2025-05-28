// test-kind: no-change

fn test() {
    match x {
        _ => {
            Foooooooooooooooooooooooo::<
                AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA,
                BBBBBBBBBBBBBBBBBBBBBBBBBBBBBB,
                CCCCCCCCCCCCCCCCCCCCCCCCCCCCCC,
            > {
                x,
                y,
                z,
            }
        }
    }
}
