pub fn is_closer_char(c: u8) -> bool {
    matches!(c, b'(' | b')' | b']' | b'}' | b'?' | b'>')
}
