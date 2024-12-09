mod ast;

use crate::writer::{Constraint, ConstraintError, ConstraintWriter, WriterSnapshot};
use rustc_data_structures::sync::Lrc;
use rustc_errors::emitter::{stderr_destination, HumanEmitter};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_lexer::TokenKind;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use rustc_span::{
    source_map::{FilePathMapping, SourceMap},
    BytePos, FileName, Pos, Span,
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

struct Formatter<'a> {
    out: ConstraintWriter,
    source: &'a str,
    pos: BytePos,
}

#[must_use]
struct FallbackChain<'a, 'b> {
    debug_name: &'static str,
    out: &'b mut Formatter<'a>,
    snapshot: FormatterSnapshot,
    result: Option<FormatResult>,
}

impl<'a> FallbackChain<'a, '_> {
    fn next(
        mut self,
        debug_name: &'static str,
        f: impl FnOnce(&mut Formatter<'a>) -> FormatResult,
    ) -> Self {
        if matches!(self.result, None | Some(Err(_))) {
            let result = f(self.out);
            match result {
                Ok(_) => info!("{}: {} succeeded", self.debug_name, debug_name),
                Err(e) => info!("{}: {} failed: {e:?}", self.debug_name, debug_name),
            }
            if let Err(_) = result {
                self.out.restore(&self.snapshot);
            }
            self.result = Some(result);
        }
        self
    }

    fn result(self) -> FormatResult {
        self.result.expect("fallback chain cannot be empty")
    }
}

impl<'a> Formatter<'a> {
    fn crate_(&mut self, crate_: &rustc_ast::ast::Crate) -> FormatResult {
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

    fn fallback_chain(&mut self, debug_name: &'static str) -> FallbackChain<'a, '_> {
        let snapshot = self.snapshot();
        FallbackChain {
            debug_name,
            out: self,
            snapshot,
            result: None,
        }
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

    fn list<T>(
        &mut self,
        kind: ListKind,
        list: &[T],
        format_item: impl Fn(&mut Formatter<'a>, &T) -> FormatResult,
    ) -> FormatResult {
        self.token_unchecked(kind.starting_brace())?;
        if list.is_empty() {
            self.token_unchecked(kind.ending_brace())?;
            return Ok(());
        }
        self.fallback_chain("list")
            .next("single line", |this| {
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                this.optional_space(kind.should_pad_contents())?;
                format_item(this, head)?;
                for item in tail {
                    this.token_unchecked(",")?;
                    this.space()?;
                    format_item(this, item)?;
                }
                this.optional_space(kind.should_pad_contents())?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .next("wrapping to fit", |this| {
                let format_item = |this: &mut Formatter<'a>, item: &T| {
                    this.with_width_limit(10, |this| format_item(this, item))
                };
                this.out.increment_indent();
                this.newline_indent()?;
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                format_item(this, head)?;
                this.token_unchecked(",")?;
                for item in tail {
                    this.fallback_chain("list item")
                        .next("same line", |this| {
                            this.space()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        })
                        .next("wrap", |this| {
                            this.newline_indent()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        })
                        .result()?;
                }
                this.out.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .next("separate lines", |this| {
                this.out.increment_indent();
                for item in list {
                    this.newline_indent()?;
                    format_item(this, item)?;
                    this.token_unchecked(",")?;
                }
                this.out.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .result()?;
        Ok(())
    }

    fn newline_indent(&mut self) -> FormatResult {
        self.skip_whitespace_and_comments();
        self.out.newline().map_err(|e| self.err(e))?;
        self.out.indent().map_err(|e| self.err(e))?;
        Ok(())
    }

    fn token_space(&mut self, token: &'static str, pos: BytePos) -> FormatResult {
        self.token(token, pos)?;
        self.space()?;
        Ok(())
    }

    fn token(&mut self, token: &str, pos: BytePos) -> FormatResult {
        assert_eq!(pos, self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn token_with_end(&mut self, token: &str, end_pos: BytePos) -> FormatResult {
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn token_unchecked(&mut self, token: &str) -> FormatResult {
        self.out.token(&token).map_err(|e| self.err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    fn token_from_source(&mut self, span: Span) -> FormatResult {
        assert_eq!(span.lo(), self.pos);
        let token = self
            .source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span");
        self.token_unchecked(token)?;
        Ok(())
    }

    fn optional_space(&mut self, is_space: bool) -> FormatResult {
        if is_space {
            self.space()?;
        } else {
            self.no_space();
        }
        Ok(())
    }

    fn err(&self, out_err: impl Into<ConstraintError>) -> FormatError {
        FormatError {
            kind: out_err.into(),
            pos: self.pos,
        }
    }

    fn space(&mut self) -> FormatResult {
        self.out.token(" ").map_err(|e| self.err(e))?;
        self.skip_whitespace_and_comments();
        Ok(())
    }

    fn skip_whitespace_and_comments(&mut self) {
        let len = rustc_lexer::tokenize(&self.source[self.pos.to_usize()..])
            .take_while(|token| {
                matches!(
                    token.kind,
                    |TokenKind::LineComment { .. }| TokenKind::BlockComment { .. }
                        | TokenKind::Whitespace
                )
            })
            .map(|token| token.len)
            .sum();
        self.pos = self.pos + BytePos::from_u32(len);
    }

    fn no_space(&mut self) {
        self.skip_whitespace_and_comments();
    }
}

enum TokenIsWhitespace {
    Yes,
    No,
    Eof,
}

pub fn format_str(source: &str, max_width: usize) -> String {
    let crate_ = parse_ast(String::from(source));
    let mut parse_tree = Formatter {
        // nodes: Vec::new();
        out: ConstraintWriter::new(max_width),
        source,
        pos: BytePos(0),
    };
    match parse_tree.crate_(&crate_) {
        Ok(()) => {}
        Err(e) => todo!("failed to format: {e:?}"),
    }
    parse_tree.out.finish()
}

fn parse_ast(string: String) -> rustc_ast::ast::Crate {
    let source_map = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let dcx = dcx(source_map.clone());
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let psess = ParseSess::with_dcx(dcx, source_map);
        let mut parser = rustc_parse::new_parser_from_source_str(
            &psess,
            FileName::anon_source_code(&string),
            string,
        )
        .unwrap();
        parser.parse_crate_mod().unwrap_or_else(|err| {
            err.emit();
            panic!("ur done");
        })
    })
}

fn dcx(source_map: Lrc<SourceMap>) -> DiagCtxt {
    let fallback_bundle = rustc_errors::fallback_fluent_bundle(
        rustc_driver::DEFAULT_LOCALE_RESOURCES.to_vec(),
        false,
    );
    let emitter = Box::new(
        HumanEmitter::new(stderr_destination(ColorConfig::Auto), fallback_bundle)
            .sm(Some(source_map)),
    );

    DiagCtxt::new(emitter)
}

#[derive(Clone, Copy, Debug)]
pub enum ListKind {
    CurlyBraces,
    SquareBraces,
    Parethesis,
}

impl ListKind {
    pub fn starting_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "{",
            ListKind::Parethesis => "(",
            ListKind::SquareBraces => "[",
        }
    }

    pub fn ending_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "}",
            ListKind::Parethesis => ")",
            ListKind::SquareBraces => "]",
        }
    }

    pub fn should_pad_contents(self) -> bool {
        match self {
            ListKind::CurlyBraces => true,
            ListKind::SquareBraces => false,
            ListKind::Parethesis => false,
        }
    }
}
