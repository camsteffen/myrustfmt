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
                panic!("Skipped token: {:?}", &self.remaining()[..token.len as usize])
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

    pub fn remaining(&self) -> &'a str {
        &self.source[self.pos.to_usize()..]
    }

    pub fn get_span(&self, span: Span) -> &'a str {
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }
}
