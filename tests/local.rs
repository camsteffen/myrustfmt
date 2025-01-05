#![feature(rustc_private)]

mod test_lib;

use crate::test_lib::{stmt_breakpoint_test, stmt_test};
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
    let source = "fn test() {let asdfasdfasdfasf=aaa;}";
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

#[test]
fn local_breakpoints() {
    stmt_breakpoint_test(
        r#"
let x = y;
        "#,
        r#"
let x =
    y;
        "#,
    );
    stmt_breakpoint_test(
        r#"
let x =
    [yyyy];
        "#,
        r#"
let x = [
    yyyy,
];
        "#,
    );
    stmt_breakpoint_test(
        r#"
let x = Struct {
    y,
};
        "#,
        r#"
let x =
    Struct {
        y,
    };
        "#,
    );
    stmt_breakpoint_test_new(
        r#"
let x = Struct {
    y,
};
=====
let x =
    Struct {
        y,
    };
  "#,
    );
}
