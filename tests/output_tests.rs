#![feature(rustc_private)]
#![feature(str_split_inclusive_remainder)]

use myrustfmt::config::Config;
use myrustfmt::format_str;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing_subscriber::EnvFilter;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

datatest_stable::harness! {
    { test = output_test, root = "tests/output_tests", pattern = r".rs" },
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG"))
        .try_init();
}

fn output_test(path: &Path) -> TestResult {
    init_tracing();
    let test = read_test(path)?;
    run_test(&test)?;
    Ok(())
}

fn read_test(path: &Path) -> TestResult<Test> {
    let string = fs::read_to_string(path)?;
    let (test_kind_raw, max_width, body) = parse_test_header(&string)?;
    let kind = parse_test_body(test_kind_raw, max_width, body)?;
    let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
    let expected_stderr_path = path.with_extension("stderr");
    let expected_stderr = match fs::read_to_string(&expected_stderr_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => panic!("Error reading stderr file: {e}"),
        Ok(content) => Some(content),
    };
    Ok(Test {
        name,
        kind,
        expected_stderr,
        expected_stderr_path,
    })
}

fn parse_test_header(string: &str) -> TestResult<(TestKindRaw, Option<u16>, &str)> {
    let mut lines = string.split_inclusive('\n');
    let mut lines_peekable = lines.by_ref().peekable();
    let mut test_kind = None::<TestKindRaw>;
    let mut max_width = None::<u16>;
    loop {
        let Some(&line) = lines_peekable.peek() else {
            break;
        };
        let Some(comment) = line.strip_prefix("//") else {
            break;
        };
        lines_peekable.next();
        let Some(comment) = comment.strip_prefix(" ") else {
            return Err("expected a leading space in header comment".into());
        };
        let comment = comment.strip_suffix('\n').unwrap();
        let (name, value) = comment
            .split_once(": ")
            .ok_or("expected \": \" in header comment")?;
        match name {
            "max-width" => {
                max_width = Some(value.parse().map_err(|_| "invalid max-width value")?);
            }
            "note" => {}
            "test-kind" => {
                if test_kind.is_some() {
                    return Err("test-kind already specified".into());
                }
                test_kind = Some(match value {
                    "before-after" => TestKindRaw::BeforeAfter,
                    "breakpoint" => TestKindRaw::Breakpoint,
                    "breakpoint-error" => TestKindRaw::BreakpointError,
                    "no-change" => TestKindRaw::NoChange,
                    _ => return Err(format!("invalid test-kind: {value:?}").into()),
                });
            }
            _ => return Err(format!("invalid name: {name:?}").into()),
        }
    }
    let Some(test_kind_raw) = test_kind else {
        return Err("expected test-kind".into());
    };
    match lines_peekable.next() {
        Some("\n") => {}
        next => return Err(
            format!("expected a blank line after header comments, found {next:?}")
                .into(),
        ),
    }
    let body = lines.remainder().unwrap_or("");
    Ok((test_kind_raw, max_width, body))
}

fn parse_test_body(
    test_kind_raw: TestKindRaw,
    max_width: Option<u16>,
    body: &str,
) -> TestResult<TestKind> {
    let kind = match test_kind_raw {
        TestKindRaw::BeforeAfter => {
            let (before, after) = parse_before_after(body)?;
            TestKind::BeforeAfter {
                before,
                after,
                max_width,
            }
        }
        TestKindRaw::Breakpoint => {
            assert!(max_width.is_none(), "cannot use max-width with this test kind");
            let (before, after) = parse_before_after(body)?;
            TestKind::Breakpoint { before, after }
        }
        TestKindRaw::BreakpointError => {
            assert!(max_width.is_none(), "cannot use max-width with this test kind");
            expect_no_after(body)?;
            TestKind::BreakpointError { formatted: body.to_owned() }
        }
        TestKindRaw::NoChange => {
            expect_no_after(body)?;
            TestKind::NoChange {
                formatted: body.to_owned(),
                max_width,
            }
        }
    };
    Ok(kind)
}

fn expect_no_after(source: &str) -> TestResult {
    for line in source.lines() {
        if let Some((_, comment)) = line.split_once("//") {
            if comment.to_ascii_lowercase().contains(":after:") {
                return Err("Unexpected control comment".into());
            }
        }
    }
    Ok(())
}

fn parse_before_after(source: &str) -> TestResult<(String, String)> {
    let delim = "\n\n// :after:\n\n";
    let Some(index) = source.find(delim) else {
        return Err(format!("Expected to find {delim:?}").into());
    };
    let before = source[..index + 1].to_owned(); // include a trailing newline
    let after = source[index + delim.len()..].to_owned();
    Ok((before, after))
}

fn run_test(test: &Test) -> TestResult {
    eprintln!("Test: {}", &test.name);
    match test.kind {
        TestKind::Breakpoint {
            ref before,
            ref after,
        } => {
            assert!(test.expected_stderr.is_none());
            breakpoint_test(before, after, None, None)?
        }
        TestKind::BreakpointError { ref formatted } => {
            let expected_stderr = test.expected_stderr.as_deref().expect(
                "breakpoint-error test should have a stderr file",
            );
            breakpoint_test(
                formatted,
                formatted,
                Some(expected_stderr),
                Some(&test.expected_stderr_path),
            )?
        }
        TestKind::NoChange {
            ref formatted,
            max_width,
        } => {
            let formatted = formatted.trim();
            format_max_width_expected(
                formatted,
                max_width,
                formatted,
                "formatted",
                test.expected_stderr.as_deref(),
                Some(&test.expected_stderr_path),
            )?;
        }
        TestKind::BeforeAfter {
            ref before,
            ref after,
            max_width,
        } => {
            let before = before.trim();
            let after = after.trim();
            format_max_width_expected(
                before,
                max_width,
                after,
                "before -> after",
                test.expected_stderr.as_deref(),
                Some(&test.expected_stderr_path),
            )?;
            format_max_width_expected(
                after,
                max_width,
                after,
                "after (idempotency)",
                test.expected_stderr.as_deref(),
                Some(&test.expected_stderr_path),
            )?;
        }
    }
    Ok(())
}

struct Test {
    name: String,
    kind: TestKind,
    expected_stderr: Option<String>,
    expected_stderr_path: PathBuf,
}

enum TestKind {
    /// A breakpoint test is for testing how formatting changes when the max width is constrained.
    /// The "before" and "after" code snippets should contain the exact same code, but the "after"
    /// string should use less width. The test is performed by formatting the "before" string with
    /// a max width that is just one character smaller than the width required for the "before"
    /// string. The result should equal the "after" string. Also, the "before" string is formatted
    /// with exactly a large enough max width to test that it is not changed.
    Breakpoint { before: String, after: String },
    BreakpointError { formatted: String },
    NoChange {
        formatted: String,
        max_width: Option<u16>,
    },
    BeforeAfter {
        before: String,
        after: String,
        max_width: Option<u16>,
    },
}

enum TestKindRaw {
    BeforeAfter,
    Breakpoint,
    BreakpointError,
    NoChange,
}

fn breakpoint_test(
    before: &str,
    after: &str,
    expected_stderr_after: Option<&str>,
    expected_stderr_after_path: Option<&Path>,
) -> TestResult {
    let before = before.trim();
    let after = after.trim();
    let initial_used_width = before.lines().map(|line| line.len() as u16).max().unwrap();
    format_max_width_expected(
        before,
        Some(initial_used_width),
        before,
        "before max width reduction",
        None,
        None,
    )?;
    println!("after width reduction...");
    format_max_width_expected(
        before,
        Some(initial_used_width - 1),
        after,
        "after max width reduction",
        expected_stderr_after,
        expected_stderr_after_path,
    )?;
    Ok(())
}

fn format_max_width_expected(
    source: &str,
    max_width: Option<u16>,
    expected: &str,
    name: &str,
    expected_stderr: Option<&str>,
    expected_stderr_path: Option<&Path>,
) -> TestResult {
    let mut config = Config::default().capture_error_output(true);
    if let Some(max_width) = max_width {
        config = config.max_width(max_width)
    }
    let result = format_str(source, config).unwrap();
    let expected = format!("{expected}\n");
    expect_formatted_equals(&result.formatted, &expected, name)?;
    let error_output = result.error_output.unwrap();
    handle_format_errors(&error_output, expected_stderr, expected_stderr_path)?;
    Ok(())
}

fn handle_format_errors(
    error_output: &str,
    expected_stderr: Option<&str>,
    expected_stderr_path: Option<&Path>,
) -> TestResult {
    let bless_path = || {
        if let Some(expected_stderr_path) = expected_stderr_path
            && std::env::var("BLESS").as_deref() == Ok("1")
        {
            Some(expected_stderr_path)
        } else {
            None
        }
    };
    match (error_output, expected_stderr) {
        ("", None) => {}
        ("", Some(_)) => {
            if let Some(expected_stderr_path) = bless_path() {
                fs::remove_file(expected_stderr_path).unwrap();
                return Ok(());
            }
            return Err("Expected errors but no errors occurred.".into());
        }
        (_, None) => {
            if let Some(expected_stderr_path) = bless_path() {
                fs::write(expected_stderr_path, error_output).unwrap();
                return Ok(());
            }
            return Err("Errors occurred".into());
        }
        (error_output, Some(expected_stderr)) => {
            if error_output != expected_stderr {
                if let Some(expected_stderr_path) = bless_path() {
                    fs::write(expected_stderr_path, error_output).unwrap();
                    return Ok(());
                }
                print_diff(expected_stderr, error_output);
                return Err("Error output does not match".into());
            }
        }
    }
    Ok(())
}

fn expect_formatted_equals(formatted: &str, expected: &str, name: &str) -> TestResult {
    if formatted == expected {
        return Ok(());
    }
    print_diff(expected, formatted);
    Err(
        format!("\"{name}\" formatted does not match expected")
            .into(),
    )
}

fn print_diff(left: &str, right: &str) {
    for line in diff::lines(left, right) {
        match line {
            diff::Result::Left(s) => println!("- {s}"),
            diff::Result::Right(s) => println!("+ {s}"),
            diff::Result::Both(s, _) => println!("  {s}"),
        }
    }
}
