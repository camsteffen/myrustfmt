#[derive(Clone, Copy, Debug, PartialEq)]
#[derive_const(Default)]
pub enum VerticalWhitespaceMode {
    /// "between items" where a blank line is allowed (e.g. between statements or items)
    Between,
    /// at the top of a file or block - a blank line is allowed below comments
    Top,
    /// at the bottom of a file or block - a blank line is allowed above comments
    Bottom,
    /// a line break where blank lines should be removed, usually breaking a construct into
    /// multiple lines that could have been on one line
    #[default]
    Break,
    /// Stop after one newline even if there is more whitespace or comments
    SingleNewline,
}

impl VerticalWhitespaceMode {
    pub fn allow_blank_line(self, is_comments_before: bool, is_comments_after: bool) -> bool {
        match self {
            VerticalWhitespaceMode::Between => true,
            VerticalWhitespaceMode::Top => is_comments_before,
            VerticalWhitespaceMode::Bottom => is_comments_after,
            VerticalWhitespaceMode::Break => is_comments_before && is_comments_after,
            VerticalWhitespaceMode::SingleNewline => false,
        }
    }
}
