pub fn is_whitespace(str: &str) -> bool {
    str.chars().all(|c| c == ';' || rustc_lexer::is_whitespace(c))
}

pub fn is_whitespace_or_semicolon(str: &str) -> bool {
    str.chars().all(|c| c == ';' || rustc_lexer::is_whitespace(c))
}
