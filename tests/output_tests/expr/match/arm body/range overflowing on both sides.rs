// test-kind: before-after

fn test() {
    match x {
        x => foo({
            let x;
        })..foo({
            let x;
        })
    }
}

// :after:

fn test() {
    match x {
        x => {
            foo({
                let x;
            })..foo({
                let x;
            })
        }
    }
}
