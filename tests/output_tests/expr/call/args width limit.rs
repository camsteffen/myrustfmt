// test-kind: before-after
// note: wrap arguments if there is more than one and they exceed 60 chars

fn test() {
    // single arg exceeds max width
    fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    // multiple args at max width
    fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaaaaaaa);
    // single arg, nested multiple args at max width
    fun(fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaa));
    // multiple args exceeds max width by 1
    fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    // single arg, nested multiple args exceeds max width by 1
    fun(fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaaa));
}

// :after:

fn test() {
    // single arg exceeds max width
    fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    // multiple args at max width
    fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaaaaaaa);
    // single arg, nested multiple args at max width
    fun(fun(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, aaaaaaaaaaaaaaaaaaaaaa));
    // multiple args exceeds max width by 1
    fun(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaaaaaaaaaaaaaa,
    );
    // single arg, nested multiple args exceeds max width by 1
    fun(fun(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        aaaaaaaaaaaaaaaaaaaaaaa,
    ));
}
