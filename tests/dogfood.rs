#![feature(rustc_private)]

use std::fs;
use tracing_test::traced_test;
use myrustfmt::{format_file, format_str};

#[traced_test]
#[test]
fn dogfood_test() {
    // panic!("{}", std::env::current_dir().unwrap().display());
    let path = "./src/lib.rs";
    let result = format_file(path);
    let original = fs::read_to_string(path).unwrap();
    assert_eq!(result, original)
}