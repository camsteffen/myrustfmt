use rustc_span::{BytePos, Pos, Span};

pub struct SourceReader<'a> {
    pub source: &'a str,
    pub pos: BytePos,
}

impl<'a> SourceReader<'a> {
    pub fn new(source: &'a str) -> SourceReader<'a> {
        SourceReader {
            source,
            pos: BytePos(0),
        }
    }

    pub fn advance(&mut self, len: usize) {
        self.pos = self.pos + BytePos::from_usize(len)
    }

    pub fn expect_pos(&self, pos: BytePos) {
        if pos == self.pos {
            return;
        }
        let token = self.next_token();
        if pos > self.pos {
            if token.len < 20 {
                panic!(
                    "Skipped token: {:?} at {}",
                    &self.remaining()[..token.len as usize],
                    self.line_col_string()
                )
            }
            panic!("Skipped token: {:?}", token.kind)
        }
        panic!(
            "Expected position to be {}, but was actually {}. Next token is {:?}.",
            pos.to_u32(),
            self.pos.to_u32(),
            token.kind
        );
    }

    fn next_token(&self) -> rustc_lexer::Token {
        rustc_lexer::tokenize(self.remaining()).next().unwrap()
    }
    
    pub fn is_next_whitespace(&self) -> bool {
        self.remaining().chars().next().is_some_and(rustc_lexer::is_whitespace)
    }

    pub fn remaining(&self) -> &'a str {
        &self.source[self.pos.to_usize()..]
    }

    pub fn get_span(&self, span: Span) -> &'a str {
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }

    pub fn line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for c in self.source[..self.pos.to_usize()].chars() {
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
