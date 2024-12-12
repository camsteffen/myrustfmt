#![feature(rustc_private)]

use myrustfmt::format_str;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn long_list_of_short_items() {
    let source =
        "fn main() { let asdfasdf = [aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa, aaaaa]; }";
    assert_eq!(
        format_str(source, 44),
        "
fn main() {
    let asdfasdf = [
        aaaaa, aaaaa, aaaaa, aaaaa, aaaaa,
        aaaaa, aaaaa, aaaaa,
    ];
}
"
        .trim_start()
    );
}

#[traced_test]
#[test]
fn long_list_of_slightly_long_items() {
    let source = "fn main() { let asdfasdf = [aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa]; }";
    assert_eq!(
        format_str(source, 20),
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
fn test_list_formats() {
    let source =
        "fn main() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";

    //     assert_eq!(
    //         format_str(source, 100),
    //         "
    // fn main() {
    //     let asdfasdf = [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];
    // }".trim_start()
    //     );
    assert_eq!(
        format_str(source, 68),
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
fn assign_wrap_long_list() {
    let source =
        "fn main() {let asdfasdfasdfasf=[aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa,aaaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 25),
        "
fn main() {
    let asdfasdfasdfasf =
        [
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
fn assign_wrap() {
    let source =
        "fn main() {let asdfasdf = [aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa,aaaaaaaaaa];}";
    assert_eq!(
        format_str(source, 72),
        "
fn main() {
    let asdfasdf =
        [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];
}
"
        .trim_start()
    );
}

#[test]
fn short_struct_pat_single_line() {
    assert_eq!(
        format_str("fn test() { let Struct { a, b, c: ccccccccc } = foo; }", 80),
        "
fn test() {
    let Struct { a, b, c: ccccccccc } = foo;
}
"
        .trim_start()
    );
}

#[test]
fn long_struct_pat_multiple_lines() {
    assert_eq!(
        format_str(
            "fn test() { let Struct { a, b, c: cccccccccc } = foo; }",
            80
        ),
        "
fn test() {
    let Struct {
        a,
        b,
        c: cccccccccc,
    } = foo;
}
"
        .trim_start()
    );
}
