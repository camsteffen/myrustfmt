use myrustfmt::config::Config;
use myrustfmt::format_str_config;

pub fn stmt_breakpoint_test(before: &str, after: &str) {
    let before = before.trim();
    let after = after.trim();
    let initial_used_width = before.lines().map(|line| line.len() as u32).max().unwrap();
    format_stmt_max_width_expected(before, Some(initial_used_width), before);
    format_stmt_max_width_expected(before, Some(initial_used_width - 1), after);
}

fn format_stmt(stmt: &str, max_width: Option<u32>) -> String {
    let (prefix, indent, suffix) = ("fn test() {\n", "    ", "}\n");
    let stmt = String::from_iter(stmt.lines().map(|s| format!("{indent}{s}\n")));
    let crate_source = format!("{prefix}{stmt}{suffix}");
    let mut config = Config::default();
    if let Some(max_width) = max_width {
        config = config.max_width(max_width + indent.len() as u32)
    }
    let formatted_crate = format_str_config(&crate_source, config)
        .unwrap()
        .expect_not_exceeded_max_width();
    let mut formatted_stmt = formatted_crate
        .strip_prefix(prefix)
        .unwrap()
        .strip_suffix(suffix)
        .unwrap()
        .lines()
        .map(|mut line| {
            for _ in 0..4 {
                match line.strip_prefix(' ') {
                    None => break,
                    Some(l) => line = l,
                }
            }
            line
        })
        .fold(String::new(), |mut acc, line| {
            acc.push_str(line);
            acc.push('\n');
            acc
        });
    formatted_stmt.pop().unwrap();
    formatted_stmt
}

pub fn format_stmt_max_width_expected(stmt: &str, max_width: Option<u32>, expected: &str) {
    let formatted = format_stmt(stmt, max_width);
    if formatted != expected {
        for line in diff::lines(expected, &formatted) {
            match line {
                diff::Result::Left(s) => println!("- {s}"),
                diff::Result::Right(s) => println!("+ {s}"),
                diff::Result::Both(s, _) => println!("  {s}"),
            }
        }
        panic!("Formatted code does not match expected");
        // panic!("Unformatted: {:?}\n  Formatted: {:?}\n   Expected: {:?}", stmt, formatted, expected);
    }
}
