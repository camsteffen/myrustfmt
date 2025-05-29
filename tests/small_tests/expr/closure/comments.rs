// test-kind: no-change

use std::process::Output;

fn test() {
    ||
        // comment
        -> Out
    {};
    ||
    // comment
    {};
    || -> Out
    // comment
    {};
}
