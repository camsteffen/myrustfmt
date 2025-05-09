// test-kind: breakpoint

fn test() {
    let xxxxxxxxxxxxx = if aaaaaaaaaa && bbbbbbbbbbb
    {
        xxxxxxxxxxxxx
    } else {
        xxxxxxxxxxxxx
    };
}

// :after:

fn test() {
    let xxxxxxxxxxxxx = if aaaaaaaaaa
        && bbbbbbbbbbb
    {
        xxxxxxxxxxxxx
    } else {
        xxxxxxxxxxxxx
    };
}
