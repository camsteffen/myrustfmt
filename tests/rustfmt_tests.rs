#![feature(rustc_private)]

use myrustfmt::format_str;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn get_test_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            get_test_files(&path, files);
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
}

fn read_significant_comments(file_name: &Path) -> HashMap<String, String> {
    let regex = regex::Regex::new(r"^\s*//\s*rustfmt-([^:]+):\s*(\S+)").unwrap();

    BufReader::new(fs::File::open(file_name).unwrap())
        .lines()
        .map(|line| line.unwrap())
        .map_while(|line| {
            regex.captures_iter(&line).next().map(|capture| {
                (
                    capture.get(1).unwrap().as_str().to_owned(),
                    capture.get(2).unwrap().as_str().to_owned(),
                )
            })
        })
        .collect()
}

#[test]
fn slow_test() {
    let mut files = Vec::new();
    get_test_files(&Path::new("./tests/rustfmt_tests"), &mut files);
    for file in files {}

    let source = r#"
mod modA {
    mod modB {
        mod modC {
            mod modD {
                mod modE {
                    fn func() {
                        state . rule (Rule :: myrule , | state | { state . sequence (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . sequence (| state | { state . optional (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) . and_then (| state | { state . repeat (| state | { state . sequence (| state | { super :: hidden :: skip (state) . and_then (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) }) }) }) }) }) }) }) }) });
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(
        format_str(source, 800).unwrap(),
        "

"
        .trim_start()
    );
}
