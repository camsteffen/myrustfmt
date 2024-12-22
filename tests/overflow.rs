#![feature(rustc_private)]

use tracing_test::traced_test;
use myrustfmt::config::Config;
use myrustfmt::format_str_config;

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
        format_str_config(source, Config::default().rustfmt_quirks(false)),
        r#"
fn test() {
    asdfasddfasdf(asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa, || {
        let x;
    });
}
"#.trim_start()
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
        format_str_config(source, Config::default().rustfmt_quirks(true)),
        r#"
fn test() {
    asdfasddfasdf(
        asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa,
        || {
            let x;
        },
    );
}
"#.trim_start()
    );
}
