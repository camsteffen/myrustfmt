#![feature(rustc_private)]

use std::error::Error;
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use myrustfmt::config::Config;
use myrustfmt::{format_str, FormatModuleResult};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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
    if has_focus {
        return Err("a test has focus: true".into());
    }
    Ok(())
}

fn small_test(test: &Test) -> TestResult {
    eprintln!("Test: {}", &test.name);
    match &test.kind {
        TestKind::Breakpoint { before, after } => {
            assert!(!test.expect_errors);
            assert!(test.max_width.is_none());
            breakpoint_test(before, after, test.in_block)?
        }
        TestKind::NoChange { formatted } => {
            let formatted = formatted.trim();
            format_max_width_expected(
                formatted,
                test.max_width,
                formatted,
                "formatted",
                test.in_block,
                test.expect_errors,
            )?;
        }
        TestKind::BeforeAfter { before, after } => {
            let before = before.trim();
            let after = after.trim();
            format_max_width_expected(
                before,
                test.max_width,
                after,
                "before -> after",
                test.in_block,
                test.expect_errors,
            )?;
            format_max_width_expected(
                after,
                test.max_width,
                after,
                "after (idempotency)",
                test.in_block,
                false,
            )?;
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
    expect_errors: bool,
    #[serde(default)]
    in_block: bool,
    max_width: Option<u32>,
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
    Breakpoint { before: String, after: String },
    NoChange { formatted: String },
    BeforeAfter { before: String, after: String },
}

fn breakpoint_test(before: &str, after: &str, in_block: bool) -> TestResult {
    let before = before.trim();
    let after = after.trim();
    let initial_used_width = before.lines().map(|line| line.len() as u32).max().unwrap();
    format_max_width_expected(
        before,
        Some(initial_used_width),
        before,
        "before max width reduction",
        in_block,
        false,
    )?;
    println!("after width reduction...");
    format_max_width_expected(
        before,
        Some(initial_used_width - 1),
        after,
        "after max width reduction",
        in_block,
        false,
    )?;
    Ok(())
}

fn format_max_width_expected(
    source: &str,
    max_width: Option<u32>,
    expected: &str,
    name: &str,
    in_block: bool,
    expect_errors: bool,
) -> TestResult {
    let formatted = if in_block {
        format_in_block(source, max_width, expect_errors)?
    } else {
        let mut config = Config::default();
        if let Some(max_width) = max_width {
            config = config.max_width(max_width)
        }
        let result = format_str(source, config).unwrap();
        handle_format_errors(result, expect_errors)?
    };
    let expected = format!("{expected}\n");
    expect_formatted_equals(&formatted, &expected, name)?;
    Ok(())
}

fn handle_format_errors(result: FormatModuleResult, expect_errors: bool) -> TestResult<String> {
    let FormatModuleResult {error_count, formatted} =result;
    match error_count {
        0 if expect_errors => Err("expected errors".into()),
        1.. if !expect_errors => Err("errors occurred".into()),
        _ => Ok(formatted)
    }
}

fn format_in_block(stmt: &str, max_width: Option<u32>, expect_errors: bool) -> TestResult<String> {
    let (prefix, indent, suffix) = ("fn test() {\n", "    ", "}\n");
    let stmt = String::from_iter(stmt.lines().map(|s| format!("{indent}{s}\n")));
    let module_source = format!("{prefix}{stmt}{suffix}");
    let mut config = Config::default();
    if let Some(max_width) = max_width {
        let max_width = max_width + indent.len() as u32;
        let min_max_width = "fn test() {".len() as u32;
        if max_width < min_max_width {
            panic!("max width must be at least {min_max_width}");
        }
        config = config.max_width(max_width);
    }
    let result = format_str(&module_source, config).unwrap();
    let result = handle_format_errors(result, expect_errors)?;
    let lines = result
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!(
            "formatted output does not have expected prefix: {:?}",
            result
        ))
        .strip_suffix(suffix)
        .unwrap_or_else(|| panic!(
            "formatted output does not have expected suffix: {:?}",
            result
        ))
        .lines();
    let mut out = String::new();
    for (i, line) in lines.enumerate() {
        if !line.is_empty() {
            out.push_str(line.strip_prefix("    ").unwrap_or_else(|| {
                panic!("line {i} is not indented\nLine: {line}\nOutput:\n{result}")
            }));
        }
        out.push('\n');
    }
    Ok(out)
}

fn expect_formatted_equals(formatted: &str, expected: &str, name: &str) -> TestResult {
    if formatted == expected {
        return Ok(());
    }
    for line in diff::lines(expected, formatted) {
        match line {
            diff::Result::Left(s) => println!("- {s}"),
            diff::Result::Right(s) => println!("+ {s}"),
            diff::Result::Both(s, _) => println!("  {s}"),
        }
    }
    Err(
        format!("\"{name}\" formatted does not match expected")
            .into(),
    )
}
