use crate::error::{ParseError, ParseErrorKind, ParseResult};
use rustc_span::{BytePos, Pos, Span};
use std::cell::Cell;

pub struct SourceReader {
    // todo use parse session source file?
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

    pub fn expect_pos(&self, pos: BytePos) -> ParseResult {
        if pos != self.pos.get() {
            return Err(ParseError::new(ParseErrorKind::ExpectedPosition(
                pos.to_usize(),
            )));
        }
        Ok(())
    }

    pub fn eat(&self, token: &str) -> Result<(), ParseError> {
        if !self.try_eat(token) {
            return Err(ParseError::new(ParseErrorKind::ExpectedToken(
                token.to_string(),
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn try_eat(&self, token: &str) -> bool {
        if !self.remaining().starts_with(token) {
            return false;
        }
        self.advance(token.len());
        true
    }

    pub fn eat_next_token(&self) -> &str {
        let token = self.next_token();
        self.advance(token.len as usize);
        &self.remaining()[..token.len as usize]
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
        self.source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }
}
