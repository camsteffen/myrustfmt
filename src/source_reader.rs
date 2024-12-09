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
        assert_eq!(pos, self.pos)
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
