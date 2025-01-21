pub fn line_col(str: &str, pos: usize) -> (u32, u32) {
    let mut line = 1;
    let mut col = 1;
    for c in str[..pos].chars() {
        col += 1;
        if c == '\n' {
            line += 1;
            col = 1;
        }
    }
    (line, col)
}
