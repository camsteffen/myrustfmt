#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;

#[test]
fn overflow_test_at_full_fn_call_width() {
    let source = r#"
fn test() {
    asdfasddfasdf(asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa, || {
        let x;
    });
}"#;
    assert_eq!(
        format_str_config(source, Config::default())
            .unwrap()
            .expect_not_exceeded_max_width(),
        r#"
fn test() {
    asdfasddfasdf(asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa, || {
        let x;
    });
}
"#
        .trim_start()
    );
}

#[test]
fn call_with_just_a_closure_can_exceed_fn_call_width() {
    let source = r#"
fn test() {
    let has_nested = items
        .iter()
        .any(|(item, _)| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
}"#;
    assert_eq!(
        format_str_config(source, Config::default())
            .unwrap()
            .expect_not_exceeded_max_width(),
        r#"
fn test() {
    let has_nested = items
        .iter()
        .any(|(item, _)| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
}
"#
        .trim_start()
    );
}
