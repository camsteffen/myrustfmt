#![feature(rustc_private)]

use myrustfmt::format_str_defaults;
use tracing_test::traced_test;

#[traced_test]
#[test]
// todo rustfmt seems to have -1 max width when method args are multiline
#[ignore]
fn dot_chain_arm_indents() {
    let source = r#"fn test() {
    match x {
        PPPPPPPPPPPPPPPPPPPPP(aaaaaaaaaaa, aaaaaaaaaa, ref aaaaaa) => aaaaaaa
            .bbbbbbbbbbbbbbbbbbbbb(
                |aaaaaaaaaaaaaaaaa| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
                || {
                    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
                },
            ),
    }
}"#;
    assert_eq!(
        format_str_defaults(source).unwrap(),
        r#"
fn test() {
    match x {
        PPPPPPPPPPPPPPPPPPPPP(aaaaaaaaaaa, aaaaaaaaaa, ref aaaaaa) => aaaaaaa
            .bbbbbbbbbbbbbbbbbbbbb(
                |aaaaaaaaaaaaaaaaa| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
                || {
                    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
                },
            ),
    }
}
"#
        .trim_start()
    );
}
