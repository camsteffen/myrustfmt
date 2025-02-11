use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext, ListStrategy};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::builder::{ListBuilderTrait, list};
use crate::ast_formatter::list::list_config::{
    ArrayListConfig, CallParamListConfig, TupleListConfig, struct_field_list_config,
};
use crate::ast_utils::plain_block;
use crate::ast_utils::postfix_expr_kind;
use crate::constraints::MultiLineConstraint;
use crate::util::cell_ext::CellExt;
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Tail::none())
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();
        match expr.kind {
            ast::ExprKind::Array(ref items) => {
                list(Braces::SQUARE, items, self.expr_list_item_fn(items))
                    .config(ArrayListConfig)
                    .overflow()
                    .tail(take_tail())
                    .format(self)?
            }
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, take_tail())?,
            postfix_expr_kind!() => self.postfix_chain(expr, take_tail())?,
            ast::ExprKind::Tup(ref items) => {
                list(Braces::PARENS, items, self.expr_list_item_fn(items))
                    .config(TupleListConfig {
                        len: items.len(),
                        single_line_max_contents_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                    })
                    .tail(take_tail())
                    .format(self)?
            }
            ast::ExprKind::Binary(op, ref left, ref right) => {
                self.binary(left, right, op, take_tail())?
            }
            ast::ExprKind::Unary(op, ref target) => {
                self.out.token(op.as_str())?;
                self.expr_tail(target, take_tail())?;
            }
            ast::ExprKind::Lit(_) => self.out.copy_span(expr.span)?,
            ast::ExprKind::Cast(ref target, ref ty) => {
                self.expr(target)?;
                self.backtrack()
                    .next(|| {
                        self.out.space_token_space("as")?;
                        self.ty(ty)?;
                        Ok(())
                    })
                    .otherwise(|| {
                        self.indented(|| {
                            self.out.newline_within_indent()?;
                            self.out.token_space("as")?;
                            self.ty(ty)?;
                            Ok(())
                        })
                    })?;
            }
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(ref pat, ref init, ..) => {
                self.out.token_space("let")?;
                self.pat(pat)?;
                self.out.space_token_space("=")?;
                self.expr_tail(init, take_tail())?;
            }
            ast::ExprKind::If(ref condition, ref block, ref else_) => {
                self.if_(condition, block, else_.as_deref(), take_tail())?
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
            ast::ExprKind::Match(ref scrutinee, ref arms, match_kind) => match match_kind {
                ast::MatchKind::Postfix => todo!(),
                ast::MatchKind::Prefix => self.match_(scrutinee, arms)?,
            },
            ast::ExprKind::Closure(ref closure) => self.closure(closure, take_tail())?,
            ast::ExprKind::Block(ref block, label) => {
                self.label(label)?;
                self.block(block)?;
            }
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(ref left, ref right, _) => {
                self.expr(left)?;
                self.out.space_token_space("=")?;
                self.expr_tail(right, take_tail())?;
            }
            ast::ExprKind::AssignOp(op, ref left, ref right) => {
                self.expr(left)?;
                self.out.space()?;
                self.out.copy_span(op.span)?;
                self.out.space()?;
                self.expr(right)?;
            }
            ast::ExprKind::Range(ref start, ref end, limits) => {
                let sigil = match limits {
                    ast::RangeLimits::Closed => "..=",
                    ast::RangeLimits::HalfOpen => "..",
                };
                self.range(start.as_deref(), sigil, end.as_deref(), take_tail())?
            }
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.qpath(qself, path, true)?,
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => {
                self.addr_of(borrow_kind, mutability)?;
                self.expr_tail(target, take_tail())?;
            }
            ast::ExprKind::Break(label, ref inner) => {
                self.out.token("break")?;
                if label.is_some() || inner.is_some() {
                    self.out.space()?;
                }
                self.label(label)?;
                if let Some(inner) = inner {
                    self.expr_tail(inner, take_tail())?;
                }
            }
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(ref target) => {
                self.out.token("return")?;
                if let Some(target) = target {
                    self.out.space()?;
                    self.expr_tail(target, take_tail())?;
                }
            }
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(ref mac_call) => self.mac_call(mac_call)?,
            ast::ExprKind::Struct(ref struct_) => self.struct_expr(struct_, take_tail())?,
            ast::ExprKind::Repeat(_, _) => todo!(),
            ast::ExprKind::Paren(ref inner) => {
                let tail = take_tail();
                self.out.token("(")?;
                self.backtrack()
                    .next(|| {
                        self.with_single_line(|| {
                            self.expr(inner)?;
                            self.out.token(")")?;
                            Ok(())
                        })?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .otherwise(|| {
                        self.embraced_after_opening(")", || self.expr(inner))?;
                        self.tail(tail)?;
                        Ok(())
                    })?;
            }
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
        if let Some(tail) = tail {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn expr_list_item_fn(
        &self,
        list: &[P<ast::Expr>],
    ) -> impl Fn(&AstFormatter, &P<ast::Expr>, &ListItemContext) -> FormatResult {
        let outer_multi_line = self.constraints().multi_line.get();
        move |af, expr, lcx| {
            af.skip_single_expr_blocks(expr, |expr| match lcx.strategy {
                ListStrategy::SingleLine
                    if outer_multi_line < MultiLineConstraint::SingleLineLists
                        && lcx.index == list.len() - 1 =>
                {
                    let multi_line_constraint = if list.len() > 1 {
                        // todo need this? and do we need this variant at all?
                        MultiLineConstraint::SingleLineLists
                        // MultiLineConstraint::SingleLineChains
                    } else {
                        MultiLineConstraint::SingleLineChains
                    };
                    af.constraints()
                        .multi_line
                        .with_replaced(dbg!(multi_line_constraint), || af.expr(expr))
                }
                ListStrategy::SeparateLines if list.len() > 1 => af
                    .backtrack()
                    .next(|| af.constraints().with_indent_middle(|| af.expr(expr)))
                    .otherwise(|| af.expr_add_block(expr)),
                _ => af.expr(expr),
            })
        }
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

    pub fn range(
        &self,
        start: Option<&ast::Expr>,
        sigil: &str,
        end: Option<&ast::Expr>,
        tail: &Tail,
    ) -> FormatResult {
        if let Some(start) = start {
            let first_line = self.out.line();
            self.expr_tail(
                start,
                &Tail::func(|af| {
                    af.out.token(sigil)?;
                    let Some(end) = end else {
                        return af.tail(tail);
                    };
                    let is_single_line = af.out.constraints().requires_indent_middle()
                        && af.out.line() != first_line;
                    af.with_single_line_opt(is_single_line, || af.expr_tail(end, tail))?;
                    Ok(())
                }),
            )?;
        } else {
            self.out.token(sigil)?;
            match end {
                None => self.tail(tail)?,
                Some(end) => self.expr_tail(end, tail)?,
            }
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

    pub fn call(&self, func: &ast::Expr, args: &[P<ast::Expr>], tail: &Tail) -> FormatResult {
        let first_line = self.out.line();
        self.expr_tail(func, &Tail::token("("))?;
        let args = || self.call_args_after_open_paren(args, tail);
        if self.out.constraints().requires_indent_middle() && self.out.line() != first_line {
            // single line and no overflow
            // this avoids code like
            // (
            //    multiline_function_expr
            // )(
            //    multi_line_args,
            // )
            self.with_single_line(|| args().format_single_line(self))?;
        } else {
            args().format(self)?;
        }
        Ok(())
    }

    pub fn call_args_after_open_paren<'ast: 'out, 'tail: 'out, 'this: 'out, 'out>(
        &'this self,
        args: &'ast [P<ast::Expr>],
        tail: &'tail Tail<'_>,
    ) -> impl ListBuilderTrait + 'out {
        list(Braces::PARENS, args, self.expr_list_item_fn(args))
            .config(CallParamListConfig)
            .omit_open_brace()
            .overflow()
            .tail(tail)
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
            let else_ = else_?;
            let ast::ExprKind::Block(else_block, _) = &else_.kind else {
                return None;
            };
            let block_expr = self.expr_only_block(block)?;
            let else_expr = self.expr_only_block(else_block)?;
            Some((block_expr, else_expr))
        };

        if !is_single_line_cond {
            multiline()?;
        } else if let Some((block_expr, else_expr)) = single_line_parts() {
            self.backtrack()
                .next_single_line(|| {
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
                .otherwise(multiline)?;
        } else {
            multiline()?;
        }
        Ok(())
    }

    pub fn token_expr_open_brace(&self, token: &str, expr: &ast::Expr) -> FormatResult<bool> {
        self.with_single_line_opt(self.constraints().requires_indent_middle(), || {
            let first_line = self.out.line();
            self.out.token_space(token)?;
            self.expr(expr)?;
            let force_newline = self.out.line() != first_line
                && self.out.with_last_line(|line| {
                    let after_indent = &line[self.out.constraints().indent.get() as usize..];
                    after_indent
                        .chars()
                        .any(|c| !matches!(c, '(' | ')' | ']' | '}' | '?' | '>'))
                });
            let newline_open_block = || {
                self.out.newline_within_indent()?;
                self.out.token("{")?;
                Ok(())
            };
            if force_newline {
                newline_open_block()?;
            } else {
                self.backtrack()
                    .next_single_line(|| self.out.space_token("{"))
                    .otherwise(newline_open_block)?;
            }
            Ok(self.out.line() == first_line)
        })
    }

    pub fn mac_call(&self, mac_call: &ast::MacCall) -> FormatResult {
        self.path(&mac_call.path, true)?;
        self.out.token("!")?;
        self.delim_args(&mac_call.args)
    }

    fn struct_expr(&self, struct_: &ast::StructExpr, tail: &Tail) -> FormatResult {
        self.qpath(&struct_.qself, &struct_.path, true)?;
        self.out.space()?;
        // todo indent middle and multi-line qpath?
        list(Braces::CURLY, &struct_.fields, Self::expr_field)
            // todo not wide enough?
            .config(struct_field_list_config(
                RUSTFMT_CONFIG_DEFAULTS.struct_lit_width,
            ))
            .rest(ListRest::from(&struct_.rest))
            .tail(tail)
            .format(self)?;
        Ok(())
    }

    fn expr_field(&self, field: &ast::ExprField, _lcx: &ListItemContext) -> FormatResult {
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

    // todo test removing and adding blocks when there are comments
    pub fn expr_add_block(&self, expr: &ast::Expr) -> FormatResult {
        self.out.token_insert("{")?;
        self.embraced_inside(|| self.expr(expr))?;
        self.out.token_insert("}")?;
        Ok(())
    }

    /// `{{{ expr }}}` -> `expr`
    pub fn skip_single_expr_blocks(
        &self,
        expr: &ast::Expr,
        format: impl FnOnce(&ast::Expr) -> FormatResult,
    ) -> FormatResult {
        match plain_block(expr).and_then(|b| self.expr_only_block(b)) {
            None => format(expr),
            Some(inner) => {
                self.out.skip_token("{")?;
                self.skip_single_expr_blocks(inner, format)?;
                self.out.skip_token("}")?;
                Ok(())
            }
        }
    }
}
