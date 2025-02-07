#![feature(rustc_private)]

mod test_lib;

use crate::test_lib::{TestResult, breakpoint_test, format_max_width_expected};
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;

datatest_stable::harness! {
    { test = small_test_file, root = "tests/small_tests", pattern = r".yaml" },
}

fn small_test_file(test_source_path: &Path) -> TestResult {
    let file = fs::File::open(test_source_path).unwrap();
    let reader = BufReader::new(file);
    let tests: Vec<Test> = serde_yaml::from_reader(reader).unwrap();
    let has_focus = tests.iter().any(|t| t.focus);
    for test in &tests {
        if !has_focus || test.focus {
            small_test(test)?;
        }
    }
    Ok(())
}

fn small_test(test: &Test) -> TestResult {
    eprintln!("Test: {}", &test.name);
    match &test.kind {
        TestKind::Breakpoint { before, after } => breakpoint_test(before, after, test.in_block)?,
        TestKind::NoChange { formatted } => {
            let formatted = formatted.trim();
            format_max_width_expected(formatted, None, formatted, "formatted", test.in_block)?;
        }
        TestKind::BeforeAfter { before, after } => {
            let before = before.trim();
            let after = after.trim();
            format_max_width_expected(before, None, after, "before -> after", test.in_block)?;
            format_max_width_expected(after, None, after, "after (idempotency)", test.in_block)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Test {
    name: String,
    #[serde(default)]
    focus: bool,
    #[serde(flatten)]
    kind: TestKind,
    #[serde(default)]
    in_block: bool,
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
