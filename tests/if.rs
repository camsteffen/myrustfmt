#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn if_open_block_on_same_line_as_condition() {
    let source = r#"
fn test() {
    if matches( variants, ast::VariantData::Unit() | ast::VariantData::Tuple() ) { let x; }
}"#;
    assert_eq!(
        format_str_config(source, Config::default().max_width(65)).unwrap(),
        r#"
fn test() {
    if matches(
        variants,
        ast::VariantData::Unit() | ast::VariantData::Tuple(),
    ) {
        let x;
    }
}
"#
        .trim_start()
    );
}
