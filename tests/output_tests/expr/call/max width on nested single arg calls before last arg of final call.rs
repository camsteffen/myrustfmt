// test-kind: before-after
// max-width: 30
// note: The max width falls inside the nested single arg calls, which are an argument that is not
// note: the last argument of the outer call.

fn test() {
    aaaaaaaaaa(bbbbb(ccccc(ddddd)), eeeee);
}

// :after:

fn test() {
    aaaaaaaaaa(
        bbbbb(ccccc(ddddd)),
        eeeee,
    );
}
