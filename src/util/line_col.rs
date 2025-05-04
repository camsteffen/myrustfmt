use crate::rustc_span::Pos;
use rustc_span::BytePos;

pub fn line_col(str: &str, pos: BytePos) -> (u32, u32) {
    let mut line = 1;
    let mut col = 1;
    for c in str[..pos.to_usize()].chars() {
        col += 1;
        if c == '\n' {
            line += 1;
            col = 1;
        }
    }
    (line, col)
}
