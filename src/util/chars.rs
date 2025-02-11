pub fn is_closer_char(c: char) -> bool {
    matches!(c, '(' | ')' | ']' | '}' | '?' | '>')
}
