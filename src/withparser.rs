use crate::format_tree::{FormatTreeNode, ListKind};
use rustc_data_structures::sync::{IntoDynSyncSend, Lrc};
use rustc_errors::emitter::{stderr_destination, DynEmitter, Emitter, HumanEmitter, SilentEmitter};
use rustc_errors::{ColorConfig, Diag, DiagCtxt, DiagInner, Level as DiagnosticLevel};
use rustc_session::parse::ParseSess;
use rustc_span::{source_map::{FilePathMapping, SourceMap}, BytePos, FileName, Pos, Span, SyntaxContext};
use rustc_ast::{ast, Const, Extern, FnHeader, FnRetTy, StrLit};
use rustc_lexer::TokenKind;
use crate::out::{Out, OutResult, OutSnapshot};

struct ParseTree<'a> {
    // nodes: Vec<FormatTreeNode>,
    out: Out,
    source: &'a str,
    pos: BytePos,
}

#[must_use]
struct FallbackChain<'a, 'b> {
    out: &'b mut ParseTree<'a>,
    snapshot: OutSnapshot,
    result: Option<OutResult>,
}

impl<'a> FallbackChain<'a, '_> {
    fn next(mut self, f: impl FnOnce(&mut ParseTree<'a>) -> OutResult) -> Self {
        if matches!(self.result, None | Some(Err(_))) {
            let result = f(self.out);
            if let Err(_) = result {
                self.out.restore(self.snapshot);
            }
            self.result = Some(result);
        }
        self
    }

    fn result(self) -> OutResult {
        self.result.expect("fallback chain cannot be empty")
    }
}

impl<'a> ParseTree<'a> {
    fn item(&mut self, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::ExternCrate(_) => {}
            ast::ItemKind::Use(_) => {}
            ast::ItemKind::Static(_) => {}
            ast::ItemKind::Const(_) => {}
            ast::ItemKind::Fn(fn_) => {
                self.fn_(fn_, item);
            }
            ast::ItemKind::Mod(_, _) => {}
            ast::ItemKind::ForeignMod(_) => {}
            ast::ItemKind::GlobalAsm(_) => {}
            ast::ItemKind::TyAlias(_) => {}
            ast::ItemKind::Enum(_, _) => {}
            ast::ItemKind::Struct(_, _) => {}
            ast::ItemKind::Union(_, _) => {}
            ast::ItemKind::Trait(_) => {}
            ast::ItemKind::TraitAlias(_, _) => {}
            ast::ItemKind::Impl(_) => {}
            ast::ItemKind::MacCall(_) => {}
            ast::ItemKind::MacroDef(_) => {}
            ast::ItemKind::Delegation(_) => {}
            ast::ItemKind::DelegationMac(_) => {}
        }
    }

    fn fn_(&mut self, fn_: &ast::Fn, item: &ast::Item) {
        let ast::Fn { defaultness, generics, sig, body, .. } = fn_;
        self.fn_sig(sig, item);
    }

    fn fn_sig(&mut self, ast::FnSig { header, decl, span }: &ast::FnSig, item: &ast::Item) {
        self.fn_header(header);
        self.token_unchecked("fn");
        self.space();
        self.ident(item.ident);
        self.no_space();
        self.fn_decl(decl);
    }

    fn ident(&mut self, ident: ast::Ident) {
        self.token_from_source(ident.span);
    }

    fn fn_header(&mut self, ast::FnHeader { safety, coroutine_kind, constness, ext }: &ast::FnHeader) {
        self.safety(safety);
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind);
        }
        self.constness(constness);
        self.extern_(ext);
    }

    fn fn_decl(&mut self, ast::FnDecl { inputs, output }: &ast::FnDecl) -> OutResult {
        self.list(ListKind::Parethesis, inputs, |this, param| this.param(param))?;
        self.space();
        self.fn_ret_ty(output)?;
        Ok(())
    }

    fn fn_ret_ty(&mut self, output: &ast::FnRetTy) -> OutResult {
        match output {
            FnRetTy::Default(_) => {}
            FnRetTy::Ty(ty) => {
                self.token_unchecked("->");
                self.space();
                self.ty(ty);
                self.space();
            }
        }
        Ok(())
    }

    fn ty(&mut self, ty: &ast::Ty) {
        todo!();
    }

    fn fallback_chain(&mut self) -> FallbackChain<'a, '_> {
        FallbackChain { out: self, snapshot: self.out.snapshot(), result: None }
    }

    fn list<T>(&mut self, kind: ListKind, list: &[T], format_item: impl Fn(&mut ParseTree<'a>, &T) -> OutResult) -> OutResult {
        self.token_unchecked(kind.starting_brace())?;
        if list.is_empty() {
            self.token_unchecked(kind.ending_brace())?;
            return Ok(());
        }
        self.fallback_chain()
            // all in one line
            .next(|this| {
                let [head @ .., tail] = list else {
                    unreachable!()
                };
                this.optional_space(kind.should_pad_contents());
                for item in head {
                    format_item(this, item)?;
                    this.token_unchecked(",")?;
                    this.space();
                }
                this.node(tail)?;
                this.optional_space(kind.should_pad_contents());
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            // block indent and wrapping as needed
            .next(|this| {
                this.increment_indent();
                this.newline_indent()?;
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                format_item(this, head)?;
                this.token_unchecked(",")?;
                for item in tail {
                    this.fallback(&[
                        // continue on the same line
                        &|this| {
                            this.space()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        },
                        // wrap to the next line
                        &|this| {
                            this.newline_indent()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        },
                    ])?;
                }
                this.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            // all on separate lines
            .next(|this| {
                this.increment_indent();
                for item in list {
                    this.newline_indent()?;
                    format_item(this, item)?;
                    this.token_unchecked(",")?;
                }
                this.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .result()?;
        Ok(())
    }

    fn constness(&mut self, constness: &ast::Const) {
        match *constness {
            ast::Const::Yes(span) => {
                self.token_space("const", span.lo());
            }
            ast::Const::No => {}
        }
    }

    fn extern_(&mut self, ext: &ast::Extern) {
        match *ext {
            ast::Extern::None => {}
            ast::Extern::Implicit(span) => {
                self.token_space("extern", span.lo());
            }
            ast::Extern::Explicit(ref abi, span) => {
                self.token_space("extern", span.lo());
                self.strlit(abi);
                self.space();
            }
        }
    }

    fn strlit(&mut self, strlit: &StrLit) {
        self.token_from_source(strlit.span);
    }

    fn safety(&mut self, safety: &ast::Safety) {
        match *safety {
            ast::Safety::Unsafe(span) => {
                self.token_space("unsafe", span.lo());
            }
            ast::Safety::Safe(span) => {
                self.token_space("safe", span.lo());
            }
            ast::Safety::Default => {}
        }
    }

    fn coroutine_kind(&mut self, coroutine_kind: &ast::CoroutineKind) {
        match *coroutine_kind {
            ast::CoroutineKind::Async { span, .. } => {
                self.token_space("async", span.lo());
            }
            ast::CoroutineKind::Gen { span, .. } => {
                self.token_space("gen", span.lo());
            }
            ast::CoroutineKind::AsyncGen { span, .. } => {
                self.token_space("async", span.lo());
                self.token_unchecked("gen");
                self.space();
            }
        }
    }

    fn token_space(&mut self, token: &'static str, pos: BytePos) -> OutResult {
        self.token(token, pos)?;
        self.space();
        Ok(())
    }

    fn token(&mut self, token: impl Into<String>, pos: BytePos) -> OutResult {
        assert_eq!(pos, self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }
    
    fn token_with_end(&mut self, token: &str, end_pos: BytePos) -> OutResult {
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn token_unchecked(&mut self, token: &str) -> OutResult {
        self.out.token(&token)?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    fn token_from_source(&mut self, span: Span) -> OutResult {
        assert_eq!(span.lo(), self.pos);
        let token = self.source.get(span.lo().to_usize()..span.hi().to_usize()).expect("source string should include the span");
        self.token_unchecked(token)?;
        Ok(())
    }

    fn optional_space(&mut self, is_space: bool) {
        if is_space {
            self.space();
        } else {
            self.no_space();
        }
    }

    fn space(&mut self) -> OutResult {
        self.out.token(" ")?;
        self.skip_whitespace_and_comments();
        Ok(())
    }

    fn skip_whitespace_and_comments(&mut self) {
        let len = rustc_lexer::tokenize(&self.source[self.pos..]).take_while(|token| {
            matches!(token.kind, 
                | TokenKind::LineComment { .. }
                | TokenKind::BlockComment { .. }
                | TokenKind::Whitespace)
        }).map(|token| token.len).sum();
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

fn make_format_tree(string: String) -> String {
    let source = "fn main() {}";
    let max_width = 1000;
    let crate_ = parse_ast(String::from(source));
    let mut parse_tree = ParseTree {
        // nodes: Vec::new();
        out: Out::new(max_width),
        source,
        pos: BytePos(0),
    };
    for item in &crate_.items {
        parse_tree.item(item);
    }
    parse_tree.out.finish()
}

fn parse_ast(string: String) -> ast::Crate {
    let source_map = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let dcx = dcx(source_map.clone());
    let psess = ParseSess::with_dcx(dcx, source_map);
    let mut parser = rustc_parse::new_parser_from_source_str(
        &psess,
        FileName::anon_source_code(&string),
        string,
    ).unwrap();
    parser.parse_crate_mod().unwrap_or_else(|err| {
        err.emit();
        panic!("ur done");
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
