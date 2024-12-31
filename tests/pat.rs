#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::{format_str, format_str_config};
use tracing_test::traced_test;

#[traced_test]
#[test]
fn struct_lit_at_max_single_line_width() {
    let source = r#"
fn test() {
    match use_tree.kind {
        ast::UseTreeKind::Nested { ref items, span: _ } => {
            let x;
        }
    }
}"#;
    assert_eq!(
        format_str_config(source, Config::default()).unwrap(),
        r#"
fn test() {
    match use_tree.kind {
        ast::UseTreeKind::Nested { ref items, span: _ } => {
            let x;
        }
    }
}
"#
        .trim_start()
    );
}

#[test]
fn long_struct_pat_multiple_lines() {
    assert_eq!(
        format_str(
            "fn test() { let Struct { a, b, c: cccccccccc } = foo; }",
            80
        )
        .unwrap(),
        "
fn test() {
    let Struct {
        a,
        b,
        c: cccccccccc,
    } = foo;
}
"
        .trim_start()
    );
}
