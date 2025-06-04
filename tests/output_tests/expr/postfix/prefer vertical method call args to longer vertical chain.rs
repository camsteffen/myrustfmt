// test-kind: breakpoint
// note: todo I don't think this test is right anymore

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
