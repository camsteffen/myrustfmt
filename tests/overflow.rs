#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;
use tracing_test::traced_test;

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

// --TODO--

#[test]
fn overflow_closure_force_block() {
    let source = r#"
fn test() {
    asdfasddfasdf(
        asdfasdfasdfasdfasdfa,
        || aaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbb,
    );
}"#;
    assert_eq!(
        format_str_config(source, Config::default().rustfmt_quirks(true)).unwrap(),
        r#"
fn test() {
    asdfasddfasdf(
        asdfasdfasdfasdfasdfasasfasfasdfasdfafasdfasdfasdfadfa,
        || aaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbb,
    );
}
"#
        .trim_start()
    );
}

fn test() {
    let x = |args| {
        match args
            .asdfasdf
            .asdfasdfas
            .asdfasdfasdf
            .asdfasdfasdfasdafs
            .asdfasd
        {
            _ => "",
        };
    }

    call(asabas, asfgwe, |args| match x.asdfasdf.asdfasdfas {
        _ => "",
    })
}
