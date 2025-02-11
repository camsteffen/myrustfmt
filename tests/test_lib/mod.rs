use myrustfmt::config::Config;
use myrustfmt::format_str_config;
use std::error::Error;

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub fn breakpoint_test(before: &str, after: &str, in_block: bool) -> TestResult {
    let before = before.trim();
    let after = after.trim();
    let initial_used_width = before
        .lines()
        .map(|line| line.len() as u32)
        .max()
        .unwrap();
    format_max_width_expected(
        before,
        Some(initial_used_width),
        before,
        "before max width reduction",
        in_block,
    )?;
    format_max_width_expected(
        before,
        Some(initial_used_width - 1),
        after,
        "after max width reduction",
        in_block,
    )?;
    Ok(())
}

fn format_in_block(stmt: &str, max_width: Option<u32>) -> String {
    let (prefix, indent, suffix) = ("fn test() {\n", "    ", "}\n");
    let stmt = String::from_iter(stmt.lines().map(|s| format!("{indent}{s}\n")));
    let crate_source = format!("{prefix}{stmt}{suffix}");
    let mut config = Config::default();
    if let Some(max_width) = max_width {
        let max_width = max_width + indent.len() as u32;
        let min_max_width = "fn test() {".len() as u32;
        if max_width < min_max_width {
            panic!("max width must be at least {min_max_width}");
        }
        config = config.max_width(max_width);
    }
    let result = format_str_config(&crate_source, config)
        .unwrap()
        .expect_not_exceeded_max_width();
    let mut formatted_stmt = result
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
        .lines()
        .enumerate()
        .fold(String::new(), |mut acc, (i, line)| {
            if !line.is_empty() {
                acc.push_str(
                    line.strip_prefix("    ")
                        .unwrap_or_else(|| panic!("line {i} is not indented: {line}")),
                );
            }
            acc.push('\n');
            acc
        });
    formatted_stmt.pop().unwrap();
    formatted_stmt
}

pub fn format_max_width_expected(
    source: &str,
    max_width: Option<u32>,
    expected: &str,
    name: &str,
    in_block: bool,
) -> TestResult {
    let formatted = if in_block {
        format_in_block(source, max_width)
    } else {
        let mut config = Config::default();
        if let Some(max_width) = max_width {
            config = config.max_width(max_width)
        }
        let mut formatted = format_str_config(source, config)
            .unwrap()
            .expect_not_exceeded_max_width();
        formatted.pop();
        formatted
    };
    expect_formatted_equals(&formatted, expected, name)?;
    Ok(())
}

fn expect_formatted_equals(formatted: &str, expected: &str, name: &str) -> TestResult {
    if formatted == expected {
        return Ok(());
    }
    for line in diff::lines(expected, &formatted) {
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
