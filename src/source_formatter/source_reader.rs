use crate::error::{parse_error_display, ParseError};
use rustc_span::{SourceFile,BytePos, Pos, Span};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;

pub struct SourceReader {
    path: Option<PathBuf>,
    pub pos: Cell<BytePos>,
    pub source_file: Arc<SourceFile>,
}

impl SourceReader {
    pub fn new(path: Option<PathBuf>, source_file: Arc<SourceFile>) -> SourceReader {
        SourceReader {
            path,
            source_file,
            pos: Cell::new(BytePos(0)),
        }
    }

    pub fn finish(self) {
        if self.pos.get().to_usize() != self.source().len() {
            // todo don't panic?
            panic!(
                "Failed to reach end of file. Next char: {:?}",
                self.source()[self.pos.get().to_usize()..]
                    .chars()
                    .next()
                    .unwrap()
            );
        }
    }
    
    pub fn pos(&self) -> BytePos {
        self.pos.get()
    }
    
    pub fn source(&self) -> &str {
        self.source_file.src.as_ref().expect("SourceFile should have src")
    }

    pub fn advance(&self, len: u32) {
        self.pos.set(self.pos.get() + BytePos::from_u32(len));
    }

    pub fn expect_pos(&self, pos: BytePos) {
        if pos != self.pos.get() {
            self.parse_error(ParseError::ExpectedPosition(pos));
        }
    }

    pub fn eat(&self, token: &'static str) {
        if !self.try_eat(token) {
            self.parse_error(ParseError::ExpectedToken(token));
        }
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
    
    pub fn goto(&self, pos: BytePos) {
        self.pos.set(pos);
    }

    fn next_token(&self) -> rustc_lexer::Token {
        rustc_lexer::tokenize(self.remaining()).next().unwrap()
    }

    pub fn remaining(&self) -> &str {
        &self.source()[self.pos.get().to_usize()..]
    }

    pub fn get_span(&self, span: Span) -> &str {
        &self.source()[span.lo().to_usize()..span.hi().to_usize()]
    }

    fn parse_error(&self, error: ParseError) -> ! {
        panic!("{}", parse_error_display(error, self.path.as_deref(), self.source(), self.pos.get()));
    }
}
