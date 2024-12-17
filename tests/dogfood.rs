#![feature(rustc_private)]

use myrustfmt::{format_file, format_str};
use std::fs;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn dogfood_test() {
    dogfood_test_file("./src/lib.rs");
    dogfood_test_file("./src/ast_formatter.rs");
    dogfood_test_file("./src/config.rs");
    dogfood_test_file("./src/constraint_writer.rs");
}

fn dogfood_test_file(path: &str) {
    let result = format_file(path);
    let original = fs::read_to_string(path).unwrap();
    assert_eq!(result, original)
}
