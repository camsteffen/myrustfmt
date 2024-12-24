#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::{format_str_config, format_str_defaults};
use tracing_test::traced_test;

#[traced_test]
#[test]
fn dot_chain_wrap_instead_of_overflow() {
    let source = r#"
fn test() {
    if self.constraints_mut().newline_budget.is_some_and(|h| h <= newlines) {
        return f(self);
    }
}"#;
    assert_eq!(
        format_str_defaults(source).unwrap(),
        r#"
fn test() {
    if self
        .constraints_mut()
        .newline_budget
        .is_some_and(|h| h <= newlines)
    {
        return f(self);
    }
}
"#
        .trim_start()
    );
}

#[test]
fn overflow_single_item_chain() {
    let source = r#"
fn test() {
    self.fallback(|| { let x; })
}"#;
    assert_eq!(
        format_str_config(source, Config::default().max_width(22)).unwrap(),
        r#"
fn test() {
    self.fallback(|| {
        let x;
    })
}
"#
        .trim_start()
    );
}

#[test]
fn overflow_first_line_with_chaining_after() {
    let source = r#"
fn test() {
    self.fallback(|| {
        let x;
    })
    .next(|| {
        let x;
    });
}"#;
    assert_eq!(
        format_str_config(source, Config::default()).unwrap(),
        r#"
fn test() {
    self.fallback(|| {
        let x;
    })
    .next(|| {
        let x;
    });
}
"#
        .trim_start()
    );
}
