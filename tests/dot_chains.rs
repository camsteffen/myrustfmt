#![feature(rustc_private)]

use myrustfmt::{format_str, format_str_defaults};
use tracing_test::traced_test;

#[traced_test]
#[test]
fn dot_chain_overflow() {
    let source = r#"
fn test() {
    if self.constraints_mut().newline_budget.is_some_and(|h| h <= newlines) {
        return f(self);
    }
}"#;
    assert_eq!(
        format_str_defaults(source),
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
