mod ast;
mod fallback_chain;
mod list;

use crate::writer::{Constraint, ConstraintError, ConstraintWriter, WriterSnapshot};
use rustc_data_structures::sync::Lrc;
use rustc_errors::emitter::{HumanEmitter, stderr_destination};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_lexer::TokenKind;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use rustc_span::{
    BytePos, FileName, Pos, Span,
    source_map::{FilePathMapping, SourceMap},
};
use tracing::info;

struct FormatterSnapshot {
    writer_snapshot: WriterSnapshot,
    pos: BytePos,
}

pub type FormatResult = Result<(), FormatError>;

#[derive(Clone, Copy, Debug)]
pub struct FormatError {
    kind: ConstraintError,
    pos: BytePos,
}

pub struct Formatter<'a> {
    out: ConstraintWriter,
    source: &'a str,
    pos: BytePos,
}

impl<'a> Formatter<'a> {
    pub fn new(source: &'a str, max_width: usize) -> Formatter<'a> {
        Formatter {
            out: ConstraintWriter::new(max_width),
            source,
            pos: BytePos(0),
        }
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn crate_(&mut self, crate_: &rustc_ast::ast::Crate) -> FormatResult {
        for item in &crate_.items {
            self.skip_whitespace_and_comments();
            self.item(item)?;
        }
        Ok(())
    }

    fn snapshot(&self) -> FormatterSnapshot {
        FormatterSnapshot {
            writer_snapshot: self.out.snapshot(),
            pos: self.pos,
        }
    }

    fn restore(&mut self, snapshot: &FormatterSnapshot) {
        self.pos = snapshot.pos;
        self.out.restore(&snapshot.writer_snapshot);
    }

    fn finally<R>(
        &mut self,
        f: impl FnOnce(&mut Formatter<'a>) -> R,
        finally: impl FnOnce(&mut Formatter<'a>),
    ) -> R {
        let result = f(self);
        finally(self);
        result
    }

    fn with_no_breaks(
        &mut self,
        f: impl FnOnce(&mut Formatter<'a>) -> FormatResult,
    ) -> FormatResult {
        self.out.push_constraint(Constraint::SingleLine);
        self.finally(f, |this| this.out.pop_constraint())
    }

    fn with_width_limit(
        &mut self,
        width_limit: usize,
        f: impl FnOnce(&mut Formatter<'a>) -> FormatResult,
    ) -> FormatResult {
        self.out.push_constraint(Constraint::SingleLineLimitWidth {
            pos: self.out.len() + width_limit,
        });
        let result = f(self);
        self.out.pop_constraint();
        result
    }

    fn newline_indent(&mut self) -> FormatResult {
        self.skip_whitespace_and_comments();
        self.out
            .newline()
            .map_err(|e| self.lift_constraint_err(e))?;
        self.out.indent().map_err(|e| self.lift_constraint_err(e))?;
        Ok(())
    }

    fn token_space(&mut self, token: &'static str, pos: BytePos) -> FormatResult {
        self.token(token, pos)?;
        self.space()?;
        Ok(())
    }

    fn token(&mut self, token: &str, pos: BytePos) -> FormatResult {
        assert_eq!(pos, self.pos);
        self.token_expect(token)?;
        Ok(())
    }

    fn token_with_end(&mut self, token: &str, end_pos: BytePos) -> FormatResult {
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_expect(token)?;
        Ok(())
    }

    fn token_expect(&mut self, token: &str) -> FormatResult {
        if !self.source[self.pos.to_usize()..].starts_with(token) {
            panic!("expected token not found");
        }
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    fn token_unchecked(&mut self, token: &str) -> FormatResult {
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    fn token_missing(&mut self, token: &str) -> FormatResult {
        self.out
            .token(&token)
            .map_err(|e| self.lift_constraint_err(e))?;
        Ok(())
    }

    fn token_maybe_missing(&mut self, token: &str) -> FormatResult {
        if self.source[self.pos.to_usize()..].starts_with(token) {
            self.token_unchecked(token)
        } else {
            self.token_missing(token)
        }
    }

    fn token_from_source(&mut self, span: Span) -> FormatResult {
        assert_eq!(span.lo(), self.pos);
        let token = self.expect_span(span);
        self.token_expect(token)?;
        Ok(())
    }
    
    fn expect_span(&self, span: Span) -> &'a str {
        self
            .source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span")
    }
    
    fn with_reserved_width(&mut self, len: usize, f: impl FnOnce(&mut Self) -> FormatResult) -> FormatResult {
        self.out.sub_max_width(len).map_err(|e| self.lift_constraint_err(e))?;
        let result = f(self);
        self.out.add_max_width(len);
        result
    }

    fn optional_space(&mut self, is_space: bool) -> FormatResult {
        if is_space {
            self.space()?;
        } else {
            self.no_space();
        }
        Ok(())
    }

    fn lift_constraint_err(&self, out_err: impl Into<ConstraintError>) -> FormatError {
        FormatError {
            kind: out_err.into(),
            pos: self.pos,
        }
    }

    fn space(&mut self) -> FormatResult {
        self.out
            .token(" ")
            .map_err(|e| self.lift_constraint_err(e))?;
        self.skip_whitespace_and_comments();
        Ok(())
    }

    fn skip_whitespace_and_comments(&mut self) {
        rustc_lexer::tokenize(&self.source[self.pos.to_usize()..])
            .take_while(|token| {
                matches!(
                    token.kind,
                    |TokenKind::LineComment { .. }| TokenKind::BlockComment { .. }
                        | TokenKind::Whitespace
                )
            })
            .for_each(|token| {
                info!("skipping whitespace: {}", token.len);
                self.pos = self.pos + BytePos::from_u32(token.len);
            });
    }

    fn no_space(&mut self) {
        self.skip_whitespace_and_comments();
    }

    fn debug_pos(&self) {
        info!("{:?}", self.source[self.pos.to_usize()..].chars().next().unwrap());
    }
    
    fn debug_span(&self, span: Span) {
        info!("{:?}", self.expect_span(span))
    }
}
