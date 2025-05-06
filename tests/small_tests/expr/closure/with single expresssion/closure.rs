// test-kind: no-change

fn test() {
    |arg| || x;
    |arg| || {
        let x;
    };
}
