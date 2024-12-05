use myrustfmt::format_tree::{FormatTreeNode, ListKind};
use myrustfmt::out::format_tree;
use tracing::{info, instrument};
use tracing_test::traced_test;

/*
#[test]
fn test_1() {
    let tree = vec![
        FormatTreeNode::Token("let"),
        FormatTreeNode::Space,
        FormatTreeNode::Token("asdfasdf"),
        FormatTreeNode::Space,
        FormatTreeNode::Token("="),
        FormatTreeNode::SpaceOrWrapIndent,
        FormatTreeNode::BreakSooner(vec![
            FormatTreeNode::List(
                ListKind::SquareBraces,
                vec![
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("bbbbb"),
                    FormatTreeNode::Token("ccccc"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ]),
    ];

    assert_eq!(
        format_tree(&tree, 1000),
        "let asdfasdf = [aaaaa, bbbbb, ccccc];"
    );
    assert_eq!(
        format_tree(&tree, 18),
        "
let asdfasdf = [
    aaaaa,
    bbbbb,
    ccccc,
];"
        .trim()
    );
    assert_eq!(
        format_tree(&tree, 15),
        "
let asdfasdf =
    [
        aaaaa,
        bbbbb,
        ccccc,
    ];"
        .trim()
    );
}

 */

#[test]
fn long_list_of_short_items() {
    /*
    let tree = vec![
        FormatTreeNode::Token("let"),
        FormatTreeNode::Space,
        FormatTreeNode::Token("asdfasdf"),
        FormatTreeNode::Space,
        FormatTreeNode::Token("="),
        FormatTreeNode::SpaceOrWrapIndent,
        FormatTreeNode::BreakSooner(vec![
            FormatTreeNode::List(
                ListKind::SquareBraces,
                vec![
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ]),
    ];

     */
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
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                    FormatTreeNode::Token("aaaaa"),
                ],
            ),
            FormatTreeNode::Token(";"),
        ],
    )];

    assert_eq!(
        format_tree(&tree, 40),
        "
let asdfasdf = [
    aaaaa, aaaaa, aaaaa, aaaaa, aaaaa,
    aaaaa, aaaaa, aaaaa,
];"
        .trim()
    );
}

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