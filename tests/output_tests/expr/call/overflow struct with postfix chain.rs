// test-kind: before-after

fn test() {
    Ok(Struct { aaaa: bbbb.cccc.dd });
    Ok(Struct { aaaa: bbbb.cccc.ddd });
}

// :after:

fn test() {
    Ok(Struct { aaaa: bbbb.cccc.dd });
    Ok(Struct {
        aaaa: bbbb.cccc.ddd,
    });
}
