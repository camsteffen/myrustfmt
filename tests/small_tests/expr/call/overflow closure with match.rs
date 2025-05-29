// test-kind: no-change

fn test() {
    foo(abc, || match x {
        _ => {}
    });
}
