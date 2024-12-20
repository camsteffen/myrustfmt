use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{ArrayListConfig, list, param_list_config, struct_field_list_config, ListRest};
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_span::source_map::Spanned;

impl<'a> AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Tail::NONE)
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: Tail<'_>) -> FormatResult {
        match expr.kind {
            ast::ExprKind::Array(ref items) => list(items, |e| self.expr(e), ArrayListConfig)
                .overflow()
                .tail(tail)
                .format(self),
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, tail),
            ast::ExprKind::Field(..) | ast::ExprKind::MethodCall(_) => {
                self.dot_chain(expr, tail, false)
            }
            ast::ExprKind::Tup(ref items) => list(
                items,
                |item| self.expr(item),
                param_list_config(Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)),
            )
            .tail(tail)
            .format(self),
            ast::ExprKind::Binary(op, ref left, ref right) => self.binop(left, op, right, tail),
            ast::ExprKind::Unary(op, ref target) => {
                self.out.token_expect(op.as_str())?;
                self.expr_tail(target, tail)?;
                Ok(())
            }
            ast::ExprKind::Lit(_) => {
                self.out.copy_span(expr.span)?;
                self.tail(tail)?;
                Ok(())
            }
            ast::ExprKind::Cast(_, _) => todo!(),
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(ref pat, ref init, ..) => {
                self.out.token_expect("let")?;
                self.out.space()?;
                self.pat(pat)?;
                self.out.space()?;
                self.out.token_expect("=")?;
                self.out.space()?;
                self.expr_tail(init, tail)?;
                Ok(())
            }
            ast::ExprKind::If(ref scrutinee, ref block, ref else_) => {
                self.if_(scrutinee, block, else_.as_deref(), tail)
            }
            ast::ExprKind::While(_, _, _) => todo!(),
            ast::ExprKind::ForLoop {
                ref pat,
                ref iter,
                ref body,
                label,
                ..
            } => {
                self.label(label)?;
                self.out.token_expect("for")?;
                self.out.space()?;
                self.pat(pat)?;
                self.out.space()?;
                self.out.token_expect("in")?;
                self.out.space()?;
                self.expr(iter)?;
                self.out.space()?;
                self.block(body, tail)?;
                Ok(())
            }
            ast::ExprKind::Loop(_, _, _) => todo!(),
            ast::ExprKind::Match(ref scrutinee, ref arms, ast::MatchKind::Prefix) => {
                self.match_(scrutinee, arms, expr, tail)
            }
            ast::ExprKind::Match(_, _, ast::MatchKind::Postfix) => todo!(),
            ast::ExprKind::Closure(ref closure) => self.closure(closure, false, tail),
            ast::ExprKind::Block(ref block, label) => {
                self.label(label)?;
                self.block(block, tail)
            }
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::Await(_, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(ref left, ref right, eq_span) => {
                self.expr(left)?;
                self.out.space()?;
                self.out.token_at("=", eq_span.lo())?;
                self.out.space()?;
                self.expr_tail(right, tail)?;
                Ok(())
            }
            ast::ExprKind::AssignOp(_, _, _) => todo!(),
            ast::ExprKind::Index(ref target, ref index, _) => {
                self.expr(target)?;
                self.out.token_expect("[")?;
                self.expr(index)?;
                self.out.token_expect("]")?;
                self.tail(tail)?;
                Ok(())
            }
            ast::ExprKind::Range(ref start, ref end, limits) => {
                self.range(start.as_deref(), end.as_deref(), limits, tail)
            }
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.qpath_end(qself, path, tail),
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => {
                self.addr_of(borrow_kind, mutability, expr)?;
                self.expr_tail(target, tail)
            }
            ast::ExprKind::Break(_, _) => todo!(),
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(ref target) => {
                self.out.token_expect("return")?;
                match target {
                    None => self.tail(tail)?,
                    Some(target) => {
                        self.out.space()?;
                        self.expr_tail(target, tail)?;
                    }
                }
                Ok(())
            }
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(ref mac_call) => self.mac_call(mac_call, tail),
            ast::ExprKind::Struct(ref struct_) => self.struct_expr(struct_, tail),
            ast::ExprKind::Repeat(_, _) => todo!(),
            ast::ExprKind::Paren(ref inner) => {
                self.out.token_at("(", expr.span.lo())?;
                self.expr(inner)?;
                self.out.token_end_at(")", expr.span.hi())?;
                self.tail(tail)?;
                Ok(())
            }
            ast::ExprKind::Try(ref target) => {
                self.expr(target)?;
                self.out.token_end_at("?", expr.span.hi())?;
                self.tail(tail)?;
                Ok(())
            }
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
    }

    fn binop(
        &self,
        left: &ast::Expr,
        op: Spanned<ast::BinOpKind>,
        right: &ast::Expr,
        tail: Tail,
    ) -> FormatResult {
        self.expr(left)?;
        self.out.space()?;
        self.out.token_at(op.node.as_str(), op.span.lo())?;
        self.out.space()?;
        self.expr_tail(right, tail)?;
        Ok(())
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
        tail: Tail<'_>,
    ) -> FormatResult {
        if let Some(start) = start {
            self.expr(start)?;
        }
        match limits {
            ast::RangeLimits::Closed => self.out.token_expect("..=")?,
            ast::RangeLimits::HalfOpen => self.out.token_expect("..")?,
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
        expr: &ast::Expr,
    ) -> FormatResult {
        match borrow_kind {
            ast::BorrowKind::Raw => todo!(),
            ast::BorrowKind::Ref => self.out.token_at("&", expr.span.lo())?,
        }
        self.mutability(mutability)?;
        Ok(())
    }

    fn call(&self, func: &ast::Expr, args: &[P<ast::Expr>], end: Tail<'_>) -> FormatResult {
        self.expr(func)?;
        let single_line_max_contents_width = RUSTFMT_CONFIG_DEFAULTS.fn_call_width;
        list(
            args,
            |arg| self.expr(arg),
            param_list_config(Some(single_line_max_contents_width)),
        )
        .overflow()
        .tail(end)
        .format(self)
    }

    fn delim_args(&self, delim_args: &ast::DelimArgs, end: Tail<'_>) -> FormatResult {
        self.out.copy_span(delim_args.dspan.entire())?;
        self.tail(end)
    }

    fn if_(
        &self,
        scrutinee: &ast::Expr,
        block: &ast::Block,
        else_: Option<&ast::Expr>,
        tail: Tail,
    ) -> FormatResult {
        self.out.token_expect("if")?;
        self.out.space()?;
        self.fallback_chain(
            |chain| {
                chain
                    .next(|| self.with_single_line(|| self.expr_tail(scrutinee, Tail::OPEN_BLOCK)));
                chain.next(|| {
                    self.expr(scrutinee)?;
                    self.out.newline_indent()?;
                    self.out.token_expect("{")?;
                    Ok(())
                });
            },
            || Ok(()),
        )?;
        match else_ {
            None => self.block_after_open_brace(block, tail)?,
            Some(else_) => {
                self.block_after_open_brace(block, Tail::NONE)?;
                self.out.space()?;
                self.out.token_expect("else")?;
                self.out.space()?;
                self.expr_tail(else_, tail)?;
            }
        }
        Ok(())
    }

    pub fn mac_call(&self, mac_call: &ast::MacCall, end: Tail<'_>) -> FormatResult {
        self.path(&mac_call.path)?;
        self.out.token_expect("!")?;
        self.delim_args(&mac_call.args, end)
    }

    fn match_(
        &self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
        expr: &ast::Expr,
        end: Tail<'_>,
    ) -> FormatResult {
        self.out.token_at("match", expr.span.lo())?;
        self.out.space()?;
        self.expr_tail(scrutinee, Tail::OPEN_BLOCK)?;
        self.indented(|| {
            for arm in arms {
                self.out.newline_indent()?;
                self.arm(arm)?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.out.token_expect("}")?;
        self.tail(end)
    }

    fn arm(&self, arm: &ast::Arm) -> FormatResult {
        self.attrs(&arm.attrs)?;
        self.pat(&arm.pat)?;
        if let Some(guard) = arm.guard.as_deref() {
            self.out.space()?;
            self.out.token_expect("if")?;
            self.out.space()?;
            self.expr(guard)?;
        }
        if let Some(body) = arm.body.as_deref() {
            self.out.space()?;
            self.out.token_expect("=>")?;
            self.out.space()?;
            self.expr(body)?;
            if self.out.char_ending_at(body.span.hi()) == b'}' {
                self.out.skip_token_if_present(",")?;
            } else {
                self.out.token_expect(",")?;
            }
        } else {
            todo!();
        }
        Ok(())
    }

    fn struct_expr(&self, struct_: &ast::StructExpr, tail: Tail<'_>) -> FormatResult {
        self.qpath(&struct_.qself, &struct_.path)?;
        self.out.space()?;
        list(
            &struct_.fields,
            |f| self.expr_field(f),
            struct_field_list_config(false),
        )
        .rest(ListRest::from(&struct_.rest))
        .tail(tail)
        .format(self)?;
        Ok(())
    }

    fn expr_field(&self, field: &ast::ExprField) -> FormatResult {
        self.attrs(&field.attrs)?;
        self.ident(field.ident)?;
        if !field.is_shorthand {
            self.out.token_expect(":")?;
            self.out.space()?;
            self.expr(&field.expr)?;
        }
        Ok(())
    }
}
