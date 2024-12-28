#![feature(rustc_private)]

use myrustfmt::format_str;
use tracing_test::traced_test;

#[test]
fn wrap_to_fit_short_items() {
    let source =
        "fn test() { let asdfasdf = [aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa]; }";
    assert_eq!(
        format_str(source, 44).unwrap(),
        "
fn test() {
    let asdfasdf = [
        aaaaa, aaaaa, aaaaa, aaaaa, aaaaa,
        aaaaa, aaaaa, aaaaa,
    ];
}
"
        .trim_start()
    );
}

#[test]
fn separate_lines() {
    let source = "fn main() { let asdfasdf = [aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa]; }";
    assert_eq!(
        format_str(source, 20).unwrap(),
        "
fn main() {
    let asdfasdf = [
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
    ];
}
"
        .trim_start()
    );
}

#[traced_test]
#[test]
fn wrap_to_fit_single_line() {
    let source =
        "fn test() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 68).unwrap(),
        "
fn test() {
    let asdfasdf = [
        aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa,
    ];
}
"
        .trim_start()
    );
}
