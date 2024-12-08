use rustc_lexer::TokenKind;
use crate::format_tree::FormatTreeNode;


struct ParseTree {}

fn make_format_tree(string: &str) -> Vec<FormatTreeNode> {
    let mut nodes = Vec::new();
    for token in rustc_lexer::tokenize(s) {
        match token.kind {
            TokenKind::LineComment { .. } => {}
            TokenKind::BlockComment { .. } => {}
            TokenKind::Whitespace => {}
            TokenKind::Ident => {}
            TokenKind::InvalidIdent => {}
            TokenKind::RawIdent => {}
            TokenKind::UnknownPrefix => {}
            TokenKind::UnknownPrefixLifetime => {}
            TokenKind::RawLifetime => {}
            TokenKind::InvalidPrefix => {}
            TokenKind::Literal { .. } => {}
            TokenKind::Lifetime { .. } => {}
            TokenKind::Semi => {}
            TokenKind::Comma => {}
            TokenKind::Dot => {}
            TokenKind::OpenParen => {}
            TokenKind::CloseParen => {}
            TokenKind::OpenBrace => {}
            TokenKind::CloseBrace => {}
            TokenKind::OpenBracket => {}
            TokenKind::CloseBracket => {}
            TokenKind::At => {}
            TokenKind::Pound => {}
            TokenKind::Tilde => {}
            TokenKind::Question => {}
            TokenKind::Colon => {}
            TokenKind::Dollar => {}
            TokenKind::Eq => {}
            TokenKind::Bang => {}
            TokenKind::Lt => {}
            TokenKind::Gt => {}
            TokenKind::Minus => {}
            TokenKind::And => {}
            TokenKind::Or => {}
            TokenKind::Plus => {}
            TokenKind::Star => {}
            TokenKind::Slash => {}
            TokenKind::Caret => {}
            TokenKind::Percent => {}
            TokenKind::Unknown => {}
            TokenKind::Eof => {}
        }
    };
    nodes
}