#![feature(rustc_private)]

mod test_lib;

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
