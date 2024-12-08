use tracing::info;
use crate::out::{Constraint, Out, OutError, OutSnapshot};
use rustc_ast::{
    ast, 
};
use rustc_data_structures::sync::{Lrc};
use rustc_errors::emitter::{stderr_destination,  HumanEmitter};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_lexer::TokenKind;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use rustc_span::{
    source_map::{FilePathMapping, SourceMap},
    BytePos, FileName, Pos, Span, 
};

struct ParseTreeSnapshot {
    out_snapshot: OutSnapshot,
    pos: BytePos,
}

pub type ParseResult = Result<(), ParseError>;

#[derive(Clone, Copy, Debug)]
pub struct ParseError {
    kind: OutError,
    pos: BytePos,
}

struct ParseTree<'a> {
    // nodes: Vec<FormatTreeNode>,
    out: Out,
    source: &'a str,
    pos: BytePos,
}

#[must_use]
struct FallbackChain<'a, 'b> {
    debug_name: &'static str,
    out: &'b mut ParseTree<'a>,
    snapshot: ParseTreeSnapshot,
    result: Option<ParseResult>,
}

// #[cfg(target_os = "x86_64")]
impl<'a> FallbackChain<'a, '_> {
    fn next(mut self, debug_name: &'static str, f: impl FnOnce(&mut ParseTree<'a>) -> ParseResult) -> Self {
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

    fn result(self) -> ParseResult {
        self.result.expect("fallback chain cannot be empty")
    }
}

impl<'a> ParseTree<'a> {
    fn crate_(&mut self, crate_: &ast::Crate) -> ParseResult {
        for item in &crate_.items {
            self.skip_whitespace_and_comments();
            self.item(item)?;
        }
        Ok(())
    }
    
    fn snapshot(&self) -> ParseTreeSnapshot {
        ParseTreeSnapshot {
            out_snapshot: self.out.snapshot(),
            pos: self.pos,
        }
    }

    fn restore(&mut self, snapshot: &ParseTreeSnapshot) {
        self.pos = snapshot.pos;
        self.out.restore(&snapshot.out_snapshot);
    }

    fn item(&mut self, item: &ast::Item) -> ParseResult {
        match &item.kind {
            ast::ItemKind::ExternCrate(_) => todo!(),
            ast::ItemKind::Use(_) => todo!(),
            ast::ItemKind::Static(_) => todo!(),
            ast::ItemKind::Const(_) => todo!(),
            ast::ItemKind::Fn(fn_) => {
                self.fn_(fn_, item)
            }
            ast::ItemKind::Mod(_, _) => todo!(),
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(_) => todo!(),
            ast::ItemKind::Enum(_, _) => todo!(),
            ast::ItemKind::Struct(_, _) => todo!(),
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(_) => todo!(),
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(_) => todo!(),
            ast::ItemKind::MacCall(_) => todo!(),
            ast::ItemKind::MacroDef(_) => todo!(),
            ast::ItemKind::Delegation(_) => todo!(),
            ast::ItemKind::DelegationMac(_) => todo!(),
        }
    }

    fn fn_(&mut self, fn_: &ast::Fn, item: &ast::Item) -> ParseResult {
        let ast::Fn {
            defaultness,
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_sig(sig, item);
        if let Some(body) = body {
            self.block(body)?;
        }
        Ok(())
    }

    fn block(&mut self, block: &ast::Block) -> ParseResult {
        self.token("{", block.span.lo())?;
        if let [first_stmt, rest_stmts @ ..] = &block.stmts[..] {
            self.out.increment_indent();
            self.newline_indent()?;
            self.stmt(first_stmt)?;
            for stmt in rest_stmts {
                self.newline_indent()?;
                self.stmt(stmt)?;
            }
            self.out.decrement_indent();
            self.newline_indent()?;
        }
        self.token_unchecked("}")?;
        Ok(())
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> ParseResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local),
            ast::StmtKind::Item(_) => Ok(()),
            ast::StmtKind::Expr(_) => Ok(()),
            ast::StmtKind::Semi(_) => Ok(()),
            ast::StmtKind::Empty => Ok(()),
            ast::StmtKind::MacCall(_) => Ok(()),
        }
    }

    fn local(&mut self, local: &ast::Local) -> ParseResult {
        let ast::Local {
            pat,
            ty,
            kind,
            attrs,
            span,
            ..
        } = local;
        self.token_space("let", span.lo())?;
        self.pat(pat)?;
        match kind {
            ast::LocalKind::Decl => {
                self.no_space();
            }
            ast::LocalKind::Init(expr) => {
                self.space()?;
                self.token_unchecked("=")?;
                self.space()?;
                self.expr(expr)?;
                self.no_space();
            }
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
        self.token_unchecked(";")?;
        Ok(())
    }

    fn expr(&mut self, expr: &ast::Expr) -> ParseResult {
        match expr.kind {
            ast::ExprKind::Array(ref items) => {
                self.list(ListKind::SquareBraces, items, |this, e| this.expr(e))
            }
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(_, _) => todo!(),
            ast::ExprKind::MethodCall(_) => todo!(),
            ast::ExprKind::Tup(_) => todo!(),
            ast::ExprKind::Binary(_, _, _) => todo!(),
            ast::ExprKind::Unary(_, _) => todo!(),
            ast::ExprKind::Lit(_) => todo!(),
            ast::ExprKind::Cast(_, _) => todo!(),
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(_, _, _, _) => todo!(),
            ast::ExprKind::If(_, _, _) => todo!(),
            ast::ExprKind::While(_, _, _) => todo!(),
            ast::ExprKind::ForLoop { .. } => todo!(),
            ast::ExprKind::Loop(_, _, _) => todo!(),
            ast::ExprKind::Match(_, _, _) => todo!(),
            ast::ExprKind::Closure(_) => todo!(),
            ast::ExprKind::Block(_, _) => todo!(),
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::Await(_, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(_, _, _) => todo!(),
            ast::ExprKind::AssignOp(_, _, _) => todo!(),
            ast::ExprKind::Field(_, _) => todo!(),
            ast::ExprKind::Index(_, _, _) => todo!(),
            ast::ExprKind::Range(_, _, _) => todo!(),
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.path(path),
            ast::ExprKind::AddrOf(_, _, _) => todo!(),
            ast::ExprKind::Break(_, _) => todo!(),
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(_) => todo!(),
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(_) => todo!(),
            ast::ExprKind::Struct(_) => todo!(),
            ast::ExprKind::Repeat(_, _) => todo!(),
            ast::ExprKind::Paren(_) => todo!(),
            ast::ExprKind::Try(_) => todo!(),
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
    }

    fn path(&mut self, path: &ast::Path) -> ParseResult {
        for segment in &path.segments {
            self.path_segment(segment)?;
        }
        Ok(())
    }

    fn path_segment(&mut self, segment: &ast::PathSegment) -> ParseResult {
        self.ident(segment.ident)?;
        if let Some(args) = &segment.args {
            todo!();
        }
        Ok(())
    }

    fn pat(&mut self, pat: &ast::Pat) -> ParseResult {
        match pat.kind {
            ast::PatKind::Wild => todo!(),
            ast::PatKind::Ident(mode, ident, ref pat) => self.ident(ident),
            ast::PatKind::Struct(_, _, _, _) => todo!(),
            ast::PatKind::TupleStruct(_, _, _) => todo!(),
            ast::PatKind::Or(_) => todo!(),
            ast::PatKind::Path(_, _) => todo!(),
            ast::PatKind::Tuple(_) => todo!(),
            ast::PatKind::Box(_) => todo!(),
            ast::PatKind::Deref(_) => todo!(),
            ast::PatKind::Ref(_, _) => todo!(),
            ast::PatKind::Lit(_) => todo!(),
            ast::PatKind::Range(_, _, _) => todo!(),
            ast::PatKind::Slice(_) => todo!(),
            ast::PatKind::Rest => todo!(),
            ast::PatKind::Never => todo!(),
            ast::PatKind::Paren(_) => todo!(),
            ast::PatKind::MacCall(_) => todo!(),
            ast::PatKind::Err(_) => todo!(),
        }
    }

    fn fn_sig(&mut self, ast::FnSig { header, decl, span }: &ast::FnSig, item: &ast::Item) {
        self.fn_header(header);
        self.token_unchecked("fn");
        self.space();
        self.ident(item.ident);
        self.no_space();
        self.fn_decl(decl);
    }

    fn ident(&mut self, ident: Ident) -> ParseResult {
        self.token_from_source(ident.span)
    }

    fn fn_header(
        &mut self,
        ast::FnHeader {
            safety,
            coroutine_kind,
            constness,
            ext,
        }: &ast::FnHeader,
    ) {
        self.safety(safety);
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind);
        }
        self.constness(constness);
        self.extern_(ext);
    }

    fn fn_decl(&mut self, ast::FnDecl { inputs, output }: &ast::FnDecl) -> ParseResult {
        self.list(ListKind::Parethesis, inputs, |this, param| {
            this.param(param)
        })?;
        self.space();
        self.fn_ret_ty(output)?;
        Ok(())
    }

    fn param(&mut self, param: &ast::Param) -> ParseResult {
        todo!()
    }

    fn fn_ret_ty(&mut self, output: &ast::FnRetTy) -> ParseResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.token_unchecked("->")?;
                self.space()?;
                self.ty(ty);
                self.space()?;
            }
        }
        Ok(())
    }

    fn ty(&mut self, ty: &ast::Ty) {
        todo!();
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
    
    fn with_width_limit(&mut self, width_limit: usize, f: impl FnOnce(&mut ParseTree<'a>) -> ParseResult) -> ParseResult {
        self.out.push_constraint(Constraint::SingleLineLimitWidth { pos: self.out.len() + width_limit });
        let result = f(self);
        self.out.pop_constraint();
        result
    }

    fn list<T>(
        &mut self,
        kind: ListKind,
        list: &[T],
        format_item: impl Fn(&mut ParseTree<'a>, &T) -> ParseResult,
    ) -> ParseResult {
        self.token_unchecked(kind.starting_brace())?;
        if list.is_empty() {
            self.token_unchecked(kind.ending_brace())?;
            return Ok(());
        }
        self.fallback_chain("list")
            .next("single line", |this| {
                let [head @ .., tail] = list else {
                    unreachable!()
                };
                this.optional_space(kind.should_pad_contents())?;
                for item in head {
                    format_item(this, item)?;
                    this.token_unchecked(",")?;
                    this.space()?;
                }
                format_item(this, tail)?;
                this.optional_space(kind.should_pad_contents())?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .next("wrapping to fit", |this| {
                let format_item = |this: &mut ParseTree<'a>, item: &T| {
                    this.with_width_limit(10, |this| {
                        format_item(this, item)
                    })
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

    fn strlit(&mut self, strlit: &ast::StrLit) {
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

    fn newline_indent(&mut self) -> ParseResult {
        self.skip_whitespace_and_comments();
        self.out.newline_indent().map_err(|e| self.err(e))
    }

    fn token_space(&mut self, token: &'static str, pos: BytePos) -> ParseResult {
        self.token(token, pos)?;
        self.space()?;
        Ok(())
    }

    fn token(&mut self, token: &str, pos: BytePos) -> ParseResult {
        assert_eq!(pos, self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn token_with_end(&mut self, token: &str, end_pos: BytePos) -> ParseResult {
        assert_eq!(end_pos - BytePos::from_usize(token.len()), self.pos);
        self.token_unchecked(token)?;
        Ok(())
    }

    fn token_unchecked(&mut self, token: &str) -> ParseResult {
        self.out.token(&token).map_err(|e| self.err(e))?;
        self.pos = self.pos + BytePos::from_usize(token.len());
        Ok(())
    }

    fn token_from_source(&mut self, span: Span) -> ParseResult {
        assert_eq!(span.lo(), self.pos);
        let token = self
            .source
            .get(span.lo().to_usize()..span.hi().to_usize())
            .expect("source string should include the span");
        self.token_unchecked(token)?;
        Ok(())
    }

    fn optional_space(&mut self, is_space: bool) -> ParseResult {
        if is_space {
            self.space()?;
        } else {
            self.no_space();
        }
        Ok(())
    }
    
    fn err(&self, out_err: OutError) -> ParseError {
        ParseError {
            kind: out_err,
            pos: self.pos,
        }
    }

    fn space(&mut self) -> ParseResult {
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
    let mut parse_tree = ParseTree {
        // nodes: Vec::new();
        out: Out::new(max_width),
        source,
        pos: BytePos(0),
    };
    match parse_tree.crate_(&crate_) {
        Ok(()) => { }
        Err(e) => todo!("failed to format: {e:?}"),
    }
    parse_tree.out.finish()
}

fn parse_ast(string: String) -> ast::Crate {
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
