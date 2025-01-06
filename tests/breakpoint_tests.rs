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
            breakpoint_test_file(Path::new(concat!("tests/breakpoint_tests/", stringify!($name), ".yaml")));
        }
        )*
    };
}

breakpoint_tests! {
    array,
    local,
}

// #[test]
// fn breakpoint_tests() {
//     breakpoint_tests_visit_path(Path::new("tests/breakpoint_tests"))
// }
//
// fn breakpoint_tests_visit_path(path: &Path) {
//     let mut paths = Vec::from_iter(
//         fs::read_dir(path)
//             .unwrap()
//             .map(|entry| entry.unwrap().path()),
//     );
//     paths.sort_unstable();
//     for path in paths {
//         if path.is_dir() {
//             breakpoint_tests_visit_path(&path);
//         } else if path.extension().is_some_and(|ext| ext == "yaml") {
//             breakpoint_test_file(&path);
//         }
//     }
// }

fn breakpoint_test_file(test_source_path: &Path) {
    println!("Running breakpoint tests in {}", test_source_path.display());
    let tests: Vec<Test> =
        serde_yaml::from_reader(BufReader::new(fs::File::open(test_source_path).unwrap())).unwrap();
    for test in &tests {
        breakpoint_test(test)
    }
}

fn breakpoint_test(test: &Test) {
    println!("Test: {}", &test.name);
    match &test.kind {
        TestKind::Breakpoint(test) => stmt_breakpoint_test(&test.before, &test.after),
        TestKind::Idempotent { formatted } => {
            let formatted = formatted.trim();
            format_stmt_max_width_expected(formatted, None, formatted)
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
    Breakpoint(BreakpointTest),
    Idempotent { formatted: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BreakpointTest {
    before: String,
    after: String,
}
