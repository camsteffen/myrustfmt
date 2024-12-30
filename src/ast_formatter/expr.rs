use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, list};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::list_config::{
    ArrayListConfig, CallParamListConfig, ParamListConfig, struct_field_list_config,
};
use crate::ast_utils::expr_only_block;
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl<'a> AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Tail::NONE)
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let mut tail_used = false;
        let mut use_tail = || {
            tail_used = true;
            tail
        };
        match expr.kind {
            ast::ExprKind::Array(ref items) => list(Braces::SQUARE, items, |e| self.expr(e))
                .config(&ArrayListConfig)
                .overflow()
                .tail(use_tail())
                .format(self)?,
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, use_tail())?,
            ast::ExprKind::Await(..)
            | ast::ExprKind::Field(..)
            | ast::ExprKind::MethodCall(_)
            | ast::ExprKind::Try(_) => self.dot_chain(expr, use_tail())?,
            ast::ExprKind::Tup(ref items) => list(Braces::PARENS, items, |item| self.expr(item))
                .config(&ParamListConfig {
                    single_line_max_contents_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                })
                .tail(use_tail())
                .format(self)?,
            ast::ExprKind::Binary(op, ref left, ref right) => {
                self.binary(left, right, op, use_tail())?
            }
            ast::ExprKind::Unary(op, ref target) => {
                self.out.token(op.as_str())?;
                self.expr_tail(target, use_tail())?;
            }
            ast::ExprKind::Lit(_) => self.out.copy_span(expr.span)?,
            ast::ExprKind::Cast(ref target, ref ty) => {
                self.expr(target)?;
                self.fallback(|| {
                    self.out.space_token_space("as")?;
                    self.ty(ty)?;
                    Ok(())
                })
                .next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.out.token_space("as")?;
                        self.ty(ty)?;
                        Ok(())
                    })
                })
                .result()?;
            }
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(ref pat, ref init, ..) => {
                self.out.token_space("let")?;
                self.pat(pat)?;
                self.out.space_token_space("=")?;
                self.expr_tail(init, use_tail())?;
            }
            ast::ExprKind::If(ref condition, ref block, ref else_) => {
                self.if_(condition, block, else_.as_deref(), use_tail())?
            }
            ast::ExprKind::While(ref condition, ref block, _label) => {
                self.while_(condition, block)?
            }
            ast::ExprKind::ForLoop {
                ref pat,
                ref iter,
                ref body,
                label,
                ..
            } => {
                self.label(label)?;
                self.out.token_space("for")?;
                self.pat(pat)?;
                self.out.space_token_space("in")?;
                self.expr(iter)?;
                self.out.space()?;
                self.block(body)?;
            }
            ast::ExprKind::Loop(ref block, label, _) => {
                self.label(label)?;
                self.out.token_space("loop")?;
                self.block(block)?;
            }
            ast::ExprKind::Match(ref scrutinee, ref arms, ast::MatchKind::Prefix) => {
                self.match_(scrutinee, arms)?
            }
            ast::ExprKind::Match(_, _, ast::MatchKind::Postfix) => todo!(),
            ast::ExprKind::Closure(ref closure) => self.closure(closure, use_tail())?,
            ast::ExprKind::Block(ref block, label) => {
                self.label(label)?;
                self.block(block)?;
            }
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(ref left, ref right, _) => {
                self.expr(left)?;
                self.out.space_token_space("=")?;
                self.expr_tail(right, use_tail())?;
            }
            ast::ExprKind::AssignOp(op, ref left, ref right) => {
                self.expr(left)?;
                self.out.space()?;
                self.out.copy_span(op.span)?;
                self.out.space()?;
                self.expr(right)?;
            }
            ast::ExprKind::Index(ref target, ref index, _) => {
                self.expr(target)?;
                self.out.token("[")?;
                let tail = use_tail();
                self.expr_tail(index, &Tail::token("]").and(tail))?;
            }
            ast::ExprKind::Range(ref start, ref end, limits) => {
                self.range(start.as_deref(), end.as_deref(), limits, use_tail())?
            }
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.qpath(qself, path, true)?,
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => {
                self.addr_of(borrow_kind, mutability)?;
                self.expr_tail(target, use_tail())?;
            }
            ast::ExprKind::Break(label, ref inner) => {
                self.out.token("break")?;
                if label.is_some() || inner.is_some() {
                    self.out.space()?;
                }
                self.label(label)?;
                if let Some(inner) = inner {
                    self.expr_tail(inner, use_tail())?;
                }
            }
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(ref target) => {
                self.out.token("return")?;
                if let Some(target) = target {
                    self.out.space()?;
                    self.expr_tail(target, use_tail())?;
                }
            }
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(ref mac_call) => self.mac_call(mac_call)?,
            ast::ExprKind::Struct(ref struct_) => self.struct_expr(struct_, use_tail())?,
            ast::ExprKind::Repeat(_, _) => todo!(),
            ast::ExprKind::Paren(ref inner) => {
                self.out.token("(")?;
                let tail = use_tail();
                self.expr_tail(inner, &Tail::token(")").and(tail))?;
            }
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
        if !tail_used {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn anon_const(&self, anon_const: &ast::AnonConst) -> FormatResult {
        self.expr(&anon_const.value)
    }

    fn label(&self, label: Option<ast::Label>) -> FormatResult {
        if let Some(label) = label {
            self.ident(label.ident)?;
            self.out.space()?;
        }
        Ok(())
    }

    fn range(
        &self,
        start: Option<&ast::Expr>,
        end: Option<&ast::Expr>,
        limits: ast::RangeLimits,
        tail: &Tail,
    ) -> FormatResult {
        if let Some(start) = start {
            self.expr(start)?;
        }
        match limits {
            ast::RangeLimits::Closed => self.out.token("..=")?,
            ast::RangeLimits::HalfOpen => self.out.token("..")?,
        }
        match end {
            None => self.tail(tail)?,
            Some(end) => self.expr_tail(end, tail)?,
        }
        Ok(())
    }

    pub fn addr_of(
        &self,
        borrow_kind: ast::BorrowKind,
        mutability: ast::Mutability,
    ) -> FormatResult {
        match borrow_kind {
            ast::BorrowKind::Raw => todo!(),
            ast::BorrowKind::Ref => self.out.token("&")?,
        }
        self.mutability(mutability)?;
        Ok(())
    }

    fn call(&self, func: &ast::Expr, args: &[P<ast::Expr>], end: &Tail) -> FormatResult {
        self.expr(func)?;
        list(Braces::PARENS, args, |arg| self.expr(arg))
            .config(&CallParamListConfig)
            .overflow()
            .tail(end)
            .format(self)
    }

    fn delim_args(&self, delim_args: &ast::DelimArgs) -> FormatResult {
        if matches!(delim_args.delim, rustc_ast::token::Delimiter::Brace) {
            self.out.space()?;
        }
        self.out.copy_span(delim_args.dspan.entire())
    }

    fn if_(
        &self,
        condition: &ast::Expr,
        block: &ast::Block,
        else_: Option<&ast::Expr>,
        tail: &Tail,
    ) -> FormatResult {
        let start_pos = self.out.last_line_len();
        let is_single_line_cond = self.token_expr_open_brace("if", condition)?;

        let multiline = || {
            self.block_after_open_brace(block)?;
            match else_ {
                None => self.tail(tail)?,
                Some(else_) => {
                    self.out.space_token_space("else")?;
                    self.expr_tail(else_, tail)?;
                }
            }
            Ok(())
        };

        let single_line_parts = || {
            let Some(block_expr) = expr_only_block(block) else {
                return None;
            };
            let Some(else_) = else_ else {
                return None;
            };
            let ast::ExprKind::Block(block, _) = &else_.kind else {
                return None;
            };
            let Some(else_expr) = expr_only_block(block) else {
                return None;
            };
            Some((block_expr, else_expr))
        };

        if !is_single_line_cond {
            multiline()?;
        } else if let Some((block_expr, else_expr)) = single_line_parts() {
            self.fallback(|| {
                self.with_single_line(|| {
                    self.with_width_limit_from_start(
                        start_pos,
                        RUSTFMT_CONFIG_DEFAULTS.single_line_if_else_max_width,
                        || {
                            self.out.space()?;
                            self.expr(block_expr)?;
                            self.out.space_token_space("}")?;
                            self.out.token_space("else")?;
                            self.out.token_space("{")?;
                            self.expr(else_expr)?;
                            self.out.space_token("}")?;
                            self.tail(tail)?;
                            Ok(())
                        },
                    )
                })
            })
            .next(multiline)
            .result()?;
        } else {
            multiline()?;
        }
        Ok(())
    }

    pub fn token_expr_open_brace(&self, token: &str, expr: &ast::Expr) -> FormatResult<bool> {
        let first_line = self.out.line();
        self.out.token_space(token)?;
        self.expr(expr)?;
        let force_newline = self.out.line() != first_line
            && self.out.with_last_line(|line| {
                let after_indent = &line[self.out.constraints().indent.get()..];
                after_indent.starts_with(' ')
                    || after_indent
                        .chars()
                        .any(|c| !matches!(c, '(' | ')' | ']' | '}' | '?' | '>'))
            });
        let newline_open_block = || {
            self.out.newline_indent()?;
            self.out.token("{")?;
            Ok(())
        };
        if force_newline {
            newline_open_block()?;
        } else {
            self.fallback(|| self.with_single_line(|| self.out.space_token("{")))
                .next(newline_open_block)
                .result()?;
        }
        Ok(self.out.line() == first_line)
    }

    pub fn mac_call(&self, mac_call: &ast::MacCall) -> FormatResult {
        self.path(&mac_call.path, true)?;
        self.out.token("!")?;
        self.delim_args(&mac_call.args)
    }

    fn struct_expr(&self, struct_: &ast::StructExpr, tail: &Tail) -> FormatResult {
        self.qpath(&struct_.qself, &struct_.path, true)?;
        self.out.space()?;
        list(Braces::CURLY, &struct_.fields, |f| self.expr_field(f))
            .config(&struct_field_list_config(
                false,
                RUSTFMT_CONFIG_DEFAULTS.struct_lit_width,
            ))
            .rest(ListRest::from(&struct_.rest))
            .tail(tail)
            .format(self)?;
        Ok(())
    }

    fn expr_field(&self, field: &ast::ExprField) -> FormatResult {
        self.with_attrs(&field.attrs, field.span, || {
            self.ident(field.ident)?;
            if !field.is_shorthand {
                self.out.token_space(":")?;
                self.expr(&field.expr)?;
            }
            Ok(())
        })
    }

    pub fn while_(&self, condition: &ast::Expr, block: &ast::Block) -> FormatResult {
        self.token_expr_open_brace("while", condition)?;
        self.block_after_open_brace(block)?;
        Ok(())
    }

    // utils

    pub fn expr_force_block(&self, expr: &ast::Expr) -> FormatResult {
        self.skip_single_expr_blocks(expr, |expr| self.add_block(|| self.expr(expr)))
    }

    pub fn add_block(&self, inside: impl FnOnce() -> FormatResult) -> FormatResult {
        self.out.token_missing("{")?;
        self.indented(|| {
            self.out.newline_indent()?;
            inside()?;
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.out.token_missing("}")?;
        Ok(())
    }

    pub fn skip_single_expr_blocks(
        &self,
        expr: &ast::Expr,
        format: impl FnOnce(&ast::Expr) -> FormatResult,
    ) -> FormatResult {
        let mut inner_expr = None;
        if let ast::ExprKind::Block(block, None) = &expr.kind {
            if matches!(block.rules, ast::BlockCheckMode::Default) {
                if let Some(expr) = expr_only_block(block) {
                    inner_expr = Some(expr);
                }
            }
        }
        if let Some(inner) = inner_expr {
            self.out.skip_token("{")?;
            self.skip_single_expr_blocks(inner, format)?;
            self.out.skip_token("}")?;
        } else {
            format(expr)?
        }
        Ok(())
    }
}
