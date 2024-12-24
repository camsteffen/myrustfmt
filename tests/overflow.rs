#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn overflow_test_at_full_fn_call_width() {
    let source = r#"
fn test() {
    asdfasddfasdf(asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa, || {
        let x;
    });
}"#;
    assert_eq!(
        format_str_config(source, Config::default().rustfmt_quirks(false)).unwrap(),
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

#[traced_test]
#[test]
fn overflow_test_at_full_fn_call_width_rustfmt_quirks() {
    let source = r#"
fn test() {
    asdfasddfasdf(asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa, || {
        let x;
    });
}"#;
    assert_eq!(
        format_str_config(source, Config::default().rustfmt_quirks(true)).unwrap(),
        r#"
fn test() {
    asdfasddfasdf(
        asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa,
        || {
            let x;
        },
    );
}
"#
        .trim_start()
    );
}

#[test]
#[traced_test]
fn call_with_just_a_closure_can_exceed_fn_call_width() {
    let source = r#"
fn test() {
    let has_nested = items
        .iter()
        .any(|(item, _)| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
}"#;
    assert_eq!(
        format_str_config(source, Config::default()).unwrap(),
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
