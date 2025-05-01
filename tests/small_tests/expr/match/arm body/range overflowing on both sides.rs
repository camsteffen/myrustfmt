// test-kind: before-after

fn test() {
    match x {
        x => [{
            let x;
        }]..[{
            let x;
        }]
    }
}

// :after:

fn test() {
    match x {
        x => {
            [{
                let x;
            }]..[{
                let x;
            }]
        }
    }
}
