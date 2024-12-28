#![feature(rustc_private)]

use myrustfmt::format_str;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn local_init_overflow() {
    let source =
        "fn main() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 68).unwrap(),
        "
fn main() {
    let asdfasdf = [
        aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa,
    ];
}
"
        .trim_start()
    );
}

#[traced_test]
#[test]
fn local_init_wrap_indent() {
    let source =
        "fn test() {let asdfasdfasdfasf=aaa;}";
    assert_eq!(
        format_str(source, 25).unwrap(),
        "
fn test() {
    let asdfasdfasdfasf =
        aaa;
}
"
        .trim_start()
    );
}
