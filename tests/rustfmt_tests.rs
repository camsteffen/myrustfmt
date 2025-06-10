// todo fixme
#![cfg(any())]
#![feature(rustc_private)]

use myrustfmt::config::Config;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

static SOURCE_PATH: &'static str = "./tests/rustfmt_tests/source";
static TARGET_PATH: &'static str = "./tests/rustfmt_tests/target";

#[test]
fn rustfmt_tests() {
    rustfmt_tests_visit_path(Path::new(SOURCE_PATH));
}

fn rustfmt_tests_visit_path(path: &Path) {
    let mut paths = Vec::from_iter(fs::read_dir(path).unwrap().map(|entry| {
        entry.unwrap().path()
    }));
    paths.sort_unstable();
    for path in paths {
        if path.is_dir() {
            rustfmt_tests_visit_path(&path);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            rustfmt_test_file(&path);
        }
    }
}

fn rustfmt_test_file(test_source_path: &Path) {
    let Some(config) = read_config_if_supported(test_source_path) else {
        println!("Skipping {}", test_source_path.display());
        return;
    };
    println!("Testing {}", test_source_path.display());
    let test_target_path = Path::new(TARGET_PATH).join(
        test_source_path.strip_prefix(SOURCE_PATH).unwrap(),
    );
    let formatted = format_file(test_source_path, config).unwrap();
    let target_expected = fs::read_to_string(test_target_path).unwrap();
    assert_eq!(formatted, target_expected);
}

fn read_config_if_supported(file_name: &Path) -> Option<Config> {
    let config_values = read_config_values(file_name);
    let mut config = Config::default();
    for (name, value) in &config_values {
        if is_unsupported_config(name) {
            return None;
        }
        config.set_str(name, value);
    }
    Some(config)
}

fn read_config_values(file_name: &Path) -> Vec<(String, String)> {
    let regex = regex::Regex::new(r"^\s*//\s*rustfmt-([^:]+):\s*(\S+)").unwrap();

    BufReader::new(fs::File::open(file_name).unwrap())
        .lines()
        .map_while(|line| {
            regex.captures(&line.unwrap()).map(|capture| {
                (capture[1].to_owned(), capture[2].to_owned())
            })
        })
        .collect()
}

fn is_unsupported_config(name: &str) -> bool {
    matches!(
        name,
        "fn_args_layout"
            | "imports_granularity"
            | "struct_field_align_threshold"
            | "trailing_comma"
    )
}
