// test-kind: before-after

fn test() {
    aaaaaaaaaaaaaaa.bbbbbbbbb.cccccccccccccc(aaaaaaaa, aaaaaaa)?;
    aaaaaaaaaaaaaaa.bbbbbbbbb.cccccccccccccc(aaaaaaaa, aaaaaaaa)?;
}

// :after:

fn test() {
    aaaaaaaaaaaaaaa.bbbbbbbbb.cccccccccccccc(aaaaaaaa, aaaaaaa)?;
    aaaaaaaaaaaaaaa
        .bbbbbbbbb
        .cccccccccccccc(aaaaaaaa, aaaaaaaa)?;
}
