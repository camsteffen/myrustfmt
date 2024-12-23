#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;
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
