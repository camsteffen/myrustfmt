#![feature(rustc_private)]

use myrustfmt::format_str_defaults;

#[test]
fn nested_groups_on_separate_line() {
    let source = r#"use aaaaaaaaaaaaaaaaaa::{aaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccc::{dddddddd, eeeeeeeeee}};"#;
    assert_eq!(
        format_str_defaults(source)
            .unwrap()
            .expect_no_errors(),
        r#"
use aaaaaaaaaaaaaaaaaa::{
    aaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbb,
    ccccccccc::{dddddddd, eeeeeeeeee},
};
"#
        .trim_start()
    );
}
