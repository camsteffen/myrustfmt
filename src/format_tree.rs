#[derive(Debug)]
pub enum FormatTreeNode {
    Token(&'static str),
    List(ListKind, Vec<FormatTreeNode>),
    // MaybeBlock(Vec<FormatTreeNode>),
    Space,
    // SpaceOrWrapIndent,
    // BreakSooner(Vec<FormatTreeNode>),
    // BreakLater(Vec<FormatTreeNode>),
    WrapIndent(Vec<FormatTreeNode>, Vec<FormatTreeNode>),
}

#[derive(Clone, Copy, Debug)]
pub enum ListKind {
    CurlyBraces,
    SquareBraces,
}

impl ListKind {
    pub fn starting_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "{",
            ListKind::SquareBraces => "[",
        }
    }

    pub fn ending_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "}",
            ListKind::SquareBraces => "]"
        }
    }

    pub fn should_pad_contents(self) -> bool {
        match self {
            ListKind::CurlyBraces => true,
            ListKind::SquareBraces => false,
        }
    }
}

impl FormatTreeNode {
    pub fn can_break(&self) -> bool {
        match self {
            FormatTreeNode::Token(_) | FormatTreeNode::Space => false,
            | FormatTreeNode::List(..)
            // | FormatTreeNode::BreakSooner(_)
            // | FormatTreeNode::BreakLater(_)
            // | FormatTreeNode::MaybeBlock(_)
            | FormatTreeNode::WrapIndent(..)
            // | FormatTreeNode::SpaceOrWrapIndent
            => true,
        }
    }
}
