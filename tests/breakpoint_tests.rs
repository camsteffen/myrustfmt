#![feature(rustc_private)]

mod test_lib;

use crate::test_lib::stmt_breakpoint_test;
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;

#[test]
fn breakpoint_tests() {
    breakpoint_tests_visit_path(Path::new("tests/breakpoint_tests"))
}

fn breakpoint_tests_visit_path(path: &Path) {
    let mut paths = Vec::from_iter(
        fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path()),
    );
    paths.sort_unstable();
    for path in paths {
        if path.is_dir() {
            breakpoint_tests_visit_path(&path);
        } else if path.extension().is_some_and(|ext| ext == "yaml") {
            breakpoint_test_file(&path);
        }
    }
}

fn breakpoint_test_file(test_source_path: &Path) {
    println!("Running breakpoint tests in {}", test_source_path.display());
    let tests: Vec<BreakpointTest> =
        serde_yaml::from_reader(BufReader::new(fs::File::open(test_source_path).unwrap())).unwrap();
    for test in &tests {
        breakpoint_test(test)
    }
}

fn breakpoint_test(test: &BreakpointTest) {
    stmt_breakpoint_test(&test.before, &test.after);
}

#[derive(Deserialize)]
struct BreakpointTest {
    before: String,
    after: String,
}
