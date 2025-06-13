// test-kind: before-after

fn test() {
    aaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbb(cccccccc, ccccccc)?;
    aaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbb(cccccccc, cccccccc)?;
}

// :after:

fn test() {
    aaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbb(cccccccc, ccccccc)?;
    aaaaaaaaaaaaaaa
        .bbbbbbbbbbbbbbbbbbbbbbbb(cccccccc, cccccccc)?;
}
