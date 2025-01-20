#![feature(rustc_private)]

mod test_lib;

use crate::test_lib::{format_stmt_max_width_expected, stmt_breakpoint_test};
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;

macro_rules! breakpoint_tests {
    ($($name:ident,)*) => {
        $(
        #[test]
        fn $name() {
            breakpoint_test_file(&Path::new("tests/small_tests").join(concat!(stringify!($name), ".yaml")));
        }
        )*
    };

}

breakpoint_tests! {
    array,
    binop,
    call,
    comments,
    chain,
    index,
    local,
    match_,
    paren,
    touchy_margins,
}

fn breakpoint_test_file(test_source_path: &Path) {
    println!("Running breakpoint tests in {}", test_source_path.display());
    let file = fs::File::open(test_source_path).unwrap();
    let reader = BufReader::new(file);
    let tests: Vec<Test> = serde_yaml::from_reader(reader).unwrap();
    for test in &tests {
        breakpoint_test(test)
    }
}

fn breakpoint_test(test: &Test) {
    println!("Test: {}", &test.name);
    match &test.kind {
        TestKind::Breakpoint { before, after } => stmt_breakpoint_test(before, after),
        TestKind::NoChange { formatted } => {
            let formatted = formatted.trim();
            format_stmt_max_width_expected(formatted, None, formatted)
        }
        TestKind::BeforeAfter { before, after } => {
            let before = before.trim();
            let after = after.trim();
            format_stmt_max_width_expected(before, None, after);
            // idempotency test
            format_stmt_max_width_expected(after, None, after);
        }
    }
}

#[derive(Deserialize)]
struct Test {
    name: String,
    #[serde(flatten)]
    kind: TestKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
enum TestKind {
    /// A breakpoint test is for testing how formatting changes when the max width is constrained.
    /// The "before" and "after" code snippets should contain the exact same code, but the "after"
    /// string should use less width. The test is performed by formatting the "before" string with
    /// a max width that is just one character smaller than the width required for the "before"
    /// string. The result should equal the "after" string. Also, the "before" string is formatted
    /// with exactly a large enough max width to test that it is not changed.
    Breakpoint {
        before: String,
        after: String,
    },
    NoChange {
        formatted: String,
    },
    BeforeAfter {
        before: String,
        after: String,
    },
}
