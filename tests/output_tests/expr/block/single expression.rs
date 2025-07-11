// test-kind: no-change

fn test() {
    { x }
    unsafe { x };
    let _ = { x };
    let _ = unsafe { x };
}
