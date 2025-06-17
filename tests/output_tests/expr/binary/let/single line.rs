// test-kind: before-after

fn test() {
    if let x = y {}
    if a && let b = c {}
    if &a && let b = c {}
    if &a() && let b = c {}
    if let a = b && c {}
}

// :after:

fn test() {
    if let x = y {}
    if a && let b = c {}
    if &a && let b = c {}
    if &a()
        && let b = c
    {}
    if let a = b
        && c
    {}
}
