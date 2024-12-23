#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_str_config;

#[test]
fn binary_operators_rustfmt_quirks() {
    let source =
        "fn main() {
        111111111111111111 - 11111111111111111 + 1111111111111111 * 2222222222222222 + 11111111111111111;
        111111111111111111 * 11111111111111111 + 1111111111111111 - 1111111111111111 + 2222222222222222 * 333333;
        }";
    assert_eq!(
        format_str_config(source, Config::default().max_width(48).rustfmt_quirks(true)).unwrap(),
        "
fn main() {
    111111111111111111 - 11111111111111111
        + 1111111111111111 * 2222222222222222
        + 11111111111111111;
    111111111111111111 * 11111111111111111
        + 1111111111111111 - 1111111111111111
        + 2222222222222222 * 333333;
}
"
        .trim_start()
    );
}

#[test]
fn binary_operators_no_rustfmt_quirks() {
    let source =
        "fn main() {
        111111111111111111 - 11111111111111111 + 1111111111111111 * 2222222222222222 + 11111111111111111;
        111111111111111111 * 11111111111111111 + 1111111111111111 - 1111111111111111 + 2222222222222222 * 333333;
        }";
    assert_eq!(
        format_str_config(
            source,
            Config::default().max_width(48).rustfmt_quirks(false)
        ).unwrap(),
        "
fn main() {
    111111111111111111
        - 11111111111111111
        + 1111111111111111 * 2222222222222222
        + 11111111111111111;
    111111111111111111 * 11111111111111111
        + 1111111111111111
        - 1111111111111111
        + 2222222222222222 * 333333;
}
"
        .trim_start()
    );
}

#[test]
fn no_overflow() {
    let source = r#"fn test() {
            let something = a == b && call_meeeeeeeeeeeeeeeeee(|line| { let x; });
        }"#;
    assert_eq!(
        format_str_config(
            source,
            Config::default()
        ).unwrap(),
        r#"
fn test() {
    let something = a == b
        && call_meeeeeeeeeeeeeeeeee(|line| {
            let x;
        });
}
"#
        .trim_start()
    );
}
