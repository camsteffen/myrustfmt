#![feature(rustc_private)]

use myrustfmt::format_str_defaults;

#[test]
fn use_tree_max_width_rustfmt_quirks() {
    let source = r#"
use aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::{aa, aaa};
use bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb::{bb, bbb};  
"#;
    assert_eq!(
        format_str_defaults(source).unwrap(),
        r#"
use aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::{
    aa, aaa,
};
use bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb::{bb, bbb};
"#
        .trim_start()
    );
}

#[test]
fn nested_groups_on_separate_line() {
    let source = r#"use aaaaaaaaaaaaaaaaaa::{aaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccc::{dddddddd, eeeeeeeeee}};"#;
    assert_eq!(
        format_str_defaults(source).unwrap(),
        r#"
use aaaaaaaaaaaaaaaaaa::{
    aaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbb,
    ccccccccc::{dddddddd, eeeeeeeeee},
};
"#
        .trim_start()
    );
}
