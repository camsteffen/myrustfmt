#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_file;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[test]
fn rustfmt_tests() {
    let source_path = Path::new("./tests/rustfmt_tests/source");
    let target_path = Path::new("./tests/rustfmt_tests/target");
    get_test_files(source_path, &|test_source_path| {
        let Some(config) = read_significant_comments_to_config_if_supported(test_source_path)
        else {
            println!("Skipping {}", test_source_path.display());
            return;
        };
        println!("Testing {}", test_source_path.display());
        let test_target_path =
            target_path.join(test_source_path.strip_prefix(source_path).unwrap());
        let formatted = format_file(test_source_path, config).unwrap();
        let target_expected = fs::read_to_string(test_target_path).unwrap();
        assert_eq!(formatted, target_expected);
    });
}

fn get_test_files(path: &Path, test: &impl Fn(&Path)) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            get_test_files(&path, test);
        } else if path.extension() == Some(OsStr::new("rs")) {
            test(&path);
        }
    }
}

fn read_significant_comments_to_config_if_supported(file_name: &Path) -> Option<Config> {
    let things = read_significant_comments(file_name);
    let mut config = Config::default();
    for (name, value) in &things {
        if is_unsupported_config(name) {
            return None;
        }
        config.set(name, value);
    }
    Some(config)
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

fn is_unsupported_config(name: &str) -> bool {
    matches!(name, "fn_args_layout" | "trailing_comma")
}
