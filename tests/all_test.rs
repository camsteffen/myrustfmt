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

/*
#[traced_test]
#[test]
fn test_list_formats() {
    let tree = vec![FormatTreeNode::WrapIndent(
        vec![
            FormatTreeNode::Token("let"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("asdfasdf"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("="),
        ],
        vec![
            FormatTreeNode::List(
                ListKind::SquareBraces,
                vec![
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ],
    )];

    assert_eq!(
        format_tree(&tree, 1000),
        "let asdfasdf = [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];"
    );
    assert_eq!(
        format_tree(&tree, 75),
        "
let asdfasdf = [
    aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa,
];"
        .trim()
    );
}

#[test]
fn assign_wrap_long_list() {
    let tree = vec![FormatTreeNode::WrapIndent(
        vec![
            FormatTreeNode::Token("let"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("asdfasdfasdfasf"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("="),
        ],
        vec![
            FormatTreeNode::List(
                ListKind::SquareBraces,
                vec![
                    FormatTreeNode::Token("aaaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaaa"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ],
    )];

    assert_eq!(
        format_tree(&tree, 22),
        "
let asdfasdfasdfasf =
    [
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
        aaaaaaaaaaa,
    ];"
            .trim()
    );
}

#[test]
fn assign_wrap() {
    let tree = vec![FormatTreeNode::WrapIndent(
        vec![
            FormatTreeNode::Token("let"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("asdfasdf"),
            FormatTreeNode::Space,
            FormatTreeNode::Token("="),
        ],
        vec![
            FormatTreeNode::List(
                ListKind::SquareBraces,
                vec![
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                    FormatTreeNode::Token("aaaaaaaaaa"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ],
    )];

    assert_eq!(
        format_tree(&tree, 70),
        "
let asdfasdf =
    [aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa, aaaaaaaaaa];"
            .trim()
    );
}

/*
fn test() {
    let aasdfsaaaaaaaaaaaaaaaaaaaaa =
        [aa, aaaaaa, aaafas, dfasdf, aaaa, aaaaaaa, aaaaa, aaa, aaaaa];
    let asdfsaaaaaaaaaaaaaaaaaaaaa = [
        aaaaa, aaaaaa, aaaafas, sdaaaaa, aaaaaaa, aaaaa, aaaaaa, aaaaa,
    ];

    {
        let aasdfsaaaaaaaaaaaaaaaaaaaaa =
            [aa, aaaaaa, aaafas, dfasdf, aaaa, aaaaaaa, aaaaa, aaa, aaaaa];
        let asdfsaaaaaaaaaaaaaaaaaaaaa =
            [aaaaa, aaaaaa, aaafas, dfadfaa, aaaaaaaaa, aaaaa, aaa, aaaaa];
    }
}


 */
*/