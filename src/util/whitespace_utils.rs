pub fn is_whitespace(str: &str) -> bool {
    str.chars().all(rustc_lexer::is_whitespace)
}
