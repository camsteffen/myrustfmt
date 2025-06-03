// test-kind: before-after

fn test() {
    let Some(_) = ({
        x;
    }) else { return };
}

// :after:

fn test() {
    let Some(_) = ({
        x;
    }) else {
        return;
    };
}
