use myrustfmt::config::Config;
use myrustfmt::format_str_config;

pub fn stmt_test(stmt: &str) {
    let stmt = stmt.trim();
    assert_eq!(format_stmt(stmt), stmt);
}

pub fn stmt_breakpoint_test(before: &str, after: &str) {
    let before = before.trim();
    let after = after.trim();
    let initial_used_width = before.lines().map(|line| line.len() as u32).max().unwrap();
    format_stmt_max_width_expected(before, Some(initial_used_width), before);
    format_stmt_max_width_expected(before, Some(initial_used_width - 1), after);
}

fn format_stmt(stmt: &str) -> String {
    format_stmt_max_width(stmt, None)
}

fn format_stmt_max_width(stmt: &str, max_width: Option<u32>) -> String {
    let (prefix, indent, suffix) = ("fn test() {\n", "    ", "\n}\n");
    let crate_source = format!("{prefix}{indent}{stmt}{suffix}");
    let mut config = Config::default();
    if let Some(max_width) = max_width {
        config = config.max_width(max_width + indent.len() as u32)
    }
    let formatted_crate = format_str_config(&crate_source, config).unwrap();
    let mut formatted_stmt = formatted_crate
        .strip_prefix(prefix)
        .unwrap()
        .strip_suffix(suffix)
        .unwrap()
        .lines()
        .map(|line| line.strip_prefix(indent).unwrap())
        .fold(String::new(), |mut acc, line| {
            acc.push_str(line);
            acc.push('\n');
            acc
        });
    formatted_stmt.pop().unwrap();
    formatted_stmt
}

pub fn format_stmt_max_width_expected(stmt: &str, max_width: Option<u32>, expected: &str) {
    let formatted = format_stmt_max_width(stmt, max_width);
    if formatted != expected {
        panic!("Unformatted: {:?}\n  Formatted: {:?}\n   Expected: {:?}", stmt, formatted, expected);
    }
}
