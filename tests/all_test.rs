#![feature(rustc_private)]

use myrustfmt::withparser::format_str;
use tracing::{info, instrument};
use tracing_test::traced_test;

#[test]
fn long_list_of_short_items() {
    let source = "fn main() { let asdfasdf = [aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa]; }";
    assert_eq!(
        format_str(source, 44),
        "
fn main() {
    let asdfasdf = [
        aaaaa, aaaaa, aaaaa, aaaaa, aaaaa,
        aaaaa, aaaaa, aaaaa,
    ];
}"
        .trim()
    );
}


#[traced_test]
#[test]
fn long_list_of_slightly_long_items() {
    let source = "fn main() { let asdfasdf = [aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa]; }";
    assert_eq!(
        format_str(source, 44),
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
}"
            .trim()
    );
}

#[traced_test]
#[test]
fn test_list_formats() {
    let source = "fn main() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";

//     assert_eq!(
//         format_str(source, 100),
//         "
// fn main() {
//     let asdfasdf = [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];
// }".trim()
//     );
    assert_eq!(
        format_str(source, 68),
        "
fn main() {
    let asdfasdf = [
        aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa,
    ];
}"
        .trim()
    );
}

#[traced_test]
#[test]
fn assign_wrap_long_list() {
    let source = "fn main() {let asdfasdfasdfasf=[aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 26),
        "
fn main() {
    let asdfasdfasdfasf =
        [
            aaaaaaaaaaa,
            aaaaaaaaaaa,
            aaaaaaaaaaa,
            aaaaaaaaaaa,
        ];
}"
            .trim()
    );
}

#[traced_test]
#[test]
fn assign_wrap() {
    let source = "fn main() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 72),
        "
fn main() {
    let asdfasdf =
        [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];
}"
            .trim()
    );
}