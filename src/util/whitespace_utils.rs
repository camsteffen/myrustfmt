use rustc_lexer::FrontmatterAllowed;
use rustc_lexer::TokenKind;

pub fn is_whitespace(str: &str) -> bool {
    str.chars().all(|c| c == ';' || rustc_lexer::is_whitespace(c))
}

pub fn is_whitespace_or_semicolon(str: &str) -> bool {
    str.chars().all(|c| c == ';' || rustc_lexer::is_whitespace(c))
}

pub fn expect_first_token_after_whitespace_and_comments(str: &str, token_kind: TokenKind) -> u32 {
    let (dist, found) = first_token_after_whitespace_and_comments(str);
    if found != token_kind {
        panic!("Expected {token_kind:?} but found {found:?}");
    }
    dist
}

pub fn first_token_after_whitespace_and_comments(str: &str) -> (u32, TokenKind) {
    let mut distance = 0;
    let mut tokens = rustc_lexer::tokenize(str, FrontmatterAllowed::No);
    let token = loop {
        let Some(token) = tokens.next() else {
            panic!("tokenize reached end without Eof");
        };
        match token.kind {
            TokenKind::BlockComment { .. }
            | TokenKind::LineComment { .. }
            | TokenKind::Whitespace => distance += token.len,
            _ => break token,
        }
    };
    (distance, token.kind)
}
