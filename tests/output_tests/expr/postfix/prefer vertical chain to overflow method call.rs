// test-kind: breakpoint

fn test() {
    rooty
        .asdf
        .asdf
        .asdfasdf(Thing { thing, thing })
}

// :after:

fn test() {
    rooty.asdf.asdf.asdfasdf(
        Thing { thing, thing },
    )
}
