use crate::error::{ParseError, panic_parse_error};
use crate::span::Span;
use crate::util::line_col::line_col;
use rustc_span::{BytePos, Pos, SourceFile};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;

// todo encapsulate more - AstFormatter probably shouldn't be accessing fields
pub struct SourceReader {
    pub path: Option<PathBuf>,
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
            self.parse_error(ParseError::UnexpectedEof);
        }
    }

    pub fn pos(&self) -> BytePos {
        self.pos.get()
    }

    pub fn source(&self) -> &str {
        self.source_file.src.as_ref().expect(
            "SourceFile should have src",
        )
    }

    pub fn advance(&self, len: u32) {
        self.pos.set(self.pos.get() + BytePos::from_u32(len));
        assert!(
            self.pos.get().to_usize() <= self.source().len(),
            "source position advanced passed EOF",
        );
    }

    pub fn eat_len(&self, len: u32) -> &str {
        let start = self.pos.get().to_usize();
        self.advance(len);
        let end = self.pos.get().to_usize();
        &self.source()[start..end]
    }

    pub fn eat_span(&self, span: Span) -> &str {
        if span.lo != self.pos.get() {
            self.parse_error(ParseError::ExpectedPosition(span.lo));
        }
        let len = span.hi.to_u32() - span.lo.to_u32();
        self.eat_len(len)
    }

    pub fn eat_token(&self, token: &'static str) {
        if !self.try_eat_token(token) {
            self.parse_error(ParseError::ExpectedToken(token));
        }
    }

    #[must_use]
    pub fn try_eat_token(&self, token: &str) -> bool {
        if !self.remaining().starts_with(token) {
            return false;
        }
        self.advance(token.len().try_into().unwrap());
        true
    }

    pub fn eat_next_token(&self) -> &str {
        let token = self.next_lexer_token();
        self.eat_len(token.len)
    }

    pub fn goto(&self, pos: BytePos) {
        self.pos.set(pos);
    }

    fn next_lexer_token(&self) -> rustc_lexer::Token {
        rustc_lexer::tokenize(self.remaining()).next().unwrap()
    }

    pub fn remaining(&self) -> &str {
        &self.source()[self.pos.get().to_usize()..]
    }

    #[track_caller]
    fn parse_error(&self, error: ParseError) -> ! {
        panic_parse_error(error, self.path.as_deref(), self.source(), self.pos.get())
    }

    #[allow(unused)]
    pub fn line_col(&self) -> (u32, u32) {
        line_col(self.source(), self.pos.get())
    }
}
