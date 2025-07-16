// test-kind: before-after
// max-width: 12

fn test() {
    break (abc);
}

// :after:

fn test() {
    break (
        abc
    );
}
