use crate::error::{ParseError, ParseErrorKind, ParseResult};
use rustc_span::{BytePos, Pos, Span};
use std::cell::Cell;
use std::rc::Rc;

pub struct SourceReader {
    pub source: Rc<String>,
    pub pos: Cell<BytePos>,
}

impl SourceReader {
    pub fn new(source: Rc<String>) -> SourceReader {
        SourceReader {
            source,
            pos: Cell::new(BytePos(0)),
        }
    }

    pub fn finish(self) {
        if self.pos.get().to_usize() != self.source.len() {
            // todo don't panic?
            panic!(
                "Failed to reach end of file. Next char: {:?}",
                self.source[self.pos.get().to_usize()..]
                    .chars()
                    .next()
                    .unwrap()
            );
        }
    }

    pub fn advance(&self, len: u32) {
        self.pos.set(self.pos.get() + BytePos::from_u32(len));
    }

    pub fn expect_pos(&self, pos: BytePos) -> ParseResult {
        if pos != self.pos.get() {
            return Err(
                ParseError::new(ParseErrorKind::ExpectedPosition(pos.to_usize())),
            );
        }
        Ok(())
    }

    pub fn eat(&self, token: &str) -> Result<(), ParseError> {
        if !self.try_eat(token) {
            return Err(
                ParseError::new(ParseErrorKind::ExpectedToken(token.to_string())),
            );
        }
        Ok(())
    }

    #[must_use]
    pub fn try_eat(&self, token: &str) -> bool {
        if !self.remaining().starts_with(token) {
            return false;
        }
        self.advance(token.len().try_into().unwrap());
        true
    }

    pub fn eat_next_token(&self) -> &str {
        let token = self.next_token();
        self.advance(token.len);
        &self.remaining()[..token.len.try_into().unwrap()]
    }

    fn next_token(&self) -> rustc_lexer::Token {
        rustc_lexer::tokenize(self.remaining()).next().unwrap()
    }

    // pub fn is_next_whitespace(&self) -> bool {
    //     self.remaining()
    //         .chars()
    //         .next()
    //         .is_some_and(rustc_lexer::is_whitespace)
    // }

    pub fn remaining(&self) -> &str {
        &self.source[self.pos.get().to_usize()..]
    }

    pub fn get_span(&self, span: Span) -> &str {
        &self.source[span.lo().to_usize()..span.hi().to_usize()]
    }
}
