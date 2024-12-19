use std::cell::Cell;
use rustc_span::{BytePos, Pos, Span};

pub struct SourceReader {
    pub source: String,
    pub pos: Cell<BytePos>,
}

impl SourceReader {
    pub fn new(source: String) -> SourceReader {
        SourceReader {
            source,
            pos: Cell::new(BytePos(0)),
        }
    }

    pub fn advance(&self, len: usize) {
        self.pos.set(self.pos.get() + BytePos::from_usize(len));
    }

    pub fn expect_pos(&self, pos: BytePos) {
        if pos != self.pos.get() {
            let token = self.next_token();
            let upcoming_len = (token.len as usize).min(20).min(self.remaining().len());
            let upcoming = &self.remaining()[..upcoming_len];
            panic!(
                "Expected position is {} bytes {}. Currently at {}. Next: \"{}\"",
                pos.to_u32().abs_diff(self.pos.get().to_u32()),
                if pos.to_u32() > self.pos.get().to_u32() { "ahead" } else { "behind" },
                self.line_col_string(),
                upcoming,
            );
        }
    }
    
    pub fn eat(&self, token: &str) {
        if !self.remaining().starts_with(token) {
            let (line, col) = self.line_col();
            panic!(
                "expected token {:?}, found {:?} at {line}:{col}",
                token,
                &self.remaining()[..10.min(self.remaining().len())],
            );
        }
        self.advance(token.len());
    }

    fn next_token(&self) -> rustc_lexer::Token {
        rustc_lexer::tokenize(self.remaining()).next().unwrap()
    }
    
    pub fn is_next_whitespace(&self) -> bool {
        self.remaining().chars().next().is_some_and(rustc_lexer::is_whitespace)
    }

    pub fn remaining(&self) -> &str {
        &self.source[self.pos.get().to_usize()..]
    }

    pub fn get_span(&self, span: Span) -> &str {
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }

    pub fn line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for c in self.source[..self.pos.get().to_usize()].chars() {
            col += 1;
            if c == '\n' {
                line += 1;
                col = 1;
            }
        }
        (line, col)
    }

    pub fn line_col_string(&self) -> String {
        let (line, col) = self.line_col();
        format!("{line}:{col}")
    }
}
