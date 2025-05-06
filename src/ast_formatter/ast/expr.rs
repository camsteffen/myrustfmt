mod binary_expr;
mod r#match;
mod postfix_chain;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::options::{ListOptions, ListWrapToFit, list_opt};
use crate::ast_formatter::list::{Braces, ListItemContext, ListStrategy};
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::debug::expr_kind_name;
use crate::ast_utils::postfix_expr_kind;
use crate::constraint_writer::ConstraintRecoveryMode;
use crate::constraints::VerticalShape;
use crate::error::{ConstraintErrorKind, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use tracing::instrument;

impl AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Tail::none())
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        if self.vertical_shape() < VerticalShape::HangingIndent && !expr.attrs.is_empty() {
            return Err(ConstraintErrorKind::NextStrategy.into());
        }
        self.with_attrs_tail(&expr.attrs, expr.span, tail, || {
            self.expr_after_attrs(expr, tail)
        })
    }

    #[instrument(name = "expr", skip_all, fields(kind=expr_kind_name(expr)))]
    pub fn expr_after_attrs(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let mut tail_opt = Some(tail);
        let mut take_tail = || tail_opt.take().unwrap();
        match expr.kind {
            ast::ExprKind::Array(ref items) => self.list(
                Braces::Square,
                items,
                self.expr_list_item(),
                expr_list_opt()
                    .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.array_width)
                    .wrap_to_fit(ListWrapToFit::Yes {
                        max_element_width: Some(
                            RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold,
                        ),
                    })
                    .tail(take_tail()),
            )?,
            ast::ExprKind::ConstBlock(ref anon_const) => {
                self.out.token_space("const")?;
                self.anon_const_tail(anon_const, take_tail())?;
            }
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, take_tail())?,
            postfix_expr_kind!() => self.postfix_chain(expr, take_tail())?,
            ast::ExprKind::Tup(ref items) => self.list(
                Braces::Parens,
                items,
                self.expr_list_item(),
                expr_list_opt()
                    .force_trailing_comma(items.len() == 1)
                    .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
                    .tail(take_tail()),
            )?,
            ast::ExprKind::Binary(op, ref left, ref right) => {
                self.binary_expr(left, right, op, take_tail())?
            }
            ast::ExprKind::Unary(op, ref target) => {
                self.out.token(op.as_str())?;
                self.expr_tail(target, take_tail())?;
            }
            ast::ExprKind::Lit(_) => self.out.copy_span(expr.span)?,
            ast::ExprKind::Cast(ref target, ref ty) => self.cast(target, ty, take_tail())?,
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
                self.label(label, true)?;
                self.out.token_space("for")?;
                self.pat(pat)?;
                self.out.space_token_space("in")?;
                self.expr(iter)?;
                self.out.space()?;
                self.block_expr(false, body)?;
            }
            ast::ExprKind::Loop(ref block, label, _) => {
                self.label(label, true)?;
                self.out.token_space("loop")?;
                self.block_expr(false, block)?;
            }
            ast::ExprKind::Match(ref scrutinee, ref arms, match_kind) => match match_kind {
                ast::MatchKind::Postfix => todo!(),
                ast::MatchKind::Prefix => self.match_(scrutinee, arms)?,
            },
            ast::ExprKind::Closure(ref closure) => self.closure(closure, take_tail())?,
            ast::ExprKind::Block(ref block, label) => {
                self.block_expr_allow_horizontal(label, block, take_tail())?
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
                self.expr_tail(right, take_tail())?;
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
                self.label(label, false)?;
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
            ast::ExprKind::Paren(ref inner) => self.paren(inner, take_tail())?,
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::UnsafeBinderCast(..) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
        if let Some(tail) = tail_opt {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn expr_list_item(
        &self,
    ) -> impl Fn(&AstFormatter, &P<ast::Expr>, &Tail, ListItemContext) -> FormatResult {
        // todo kinda hacky
        let vertical_outer = self.vertical_shape();

        move |af, expr, tail, lcx| {
            af.skip_single_expr_blocks_tail(expr, tail, |expr, tail| {
                let format = || af.expr_tail(expr, tail);
                match lcx.strategy {
                    // overflow last item
                    ListStrategy::Horizontal
                        if vertical_outer >= VerticalShape::List && lcx.is_last() =>
                    {
                        // override the multi-line shape to be less strict than SingleLine
                        let shape = if lcx.len > 1 {
                            // don't overflow nested lists when the outer list has multiple items
                            VerticalShape::BlockLike
                        } else {
                            VerticalShape::List
                        };
                        // todo avoid replace?
                        af.with_replace_vertical_shape(shape, format)?;
                        Ok(())
                    }
                    // on separate lines, enforce IndentMiddle by adding a block
                    ListStrategy::Vertical if lcx.len > 1 => {
                        af.backtrack()
                            // If it's too wide, adding a block won't help.
                            // The block is only for ensuring a "hanging indent"-compliant shape.
                            .next_with_constraint_recovery_mode(
                                ConstraintRecoveryMode::Newline,
                                || af.with_vertical_shape_min(VerticalShape::HangingIndent, format),
                            )
                            .otherwise(|| {
                                af.expr_add_block(expr)?;
                                af.tail(tail)?;
                                Ok(())
                            })
                    }
                    _ => format(),
                }
            })?;
            // todo I wish this were more symmetrical with Tail being passed in
            //   maybe introduce a more specific Tail type for list comma
            self.out.skip_token_if_present(",")?;
            Ok(())
        }
    }

    pub fn anon_const(&self, anon_const: &ast::AnonConst) -> FormatResult {
        self.expr(&anon_const.value)
    }

    pub fn anon_const_tail(&self, anon_const: &ast::AnonConst, tail: &Tail) -> FormatResult {
        self.expr_tail(&anon_const.value, tail)
    }

    pub fn label(&self, label: Option<ast::Label>, has_colon: bool) -> FormatResult {
        if let Some(label) = label {
            self.ident(label.ident)?;
            if has_colon {
                self.out.token(":")?;
            }
            self.out.space()?;
        }
        Ok(())
    }

    fn cast(&self, target: &ast::Expr, ty: &ast::Ty, tail: &Tail) -> FormatResult {
        self.expr(target)?;
        self.backtrack()
            .next(|| {
                self.out.space_token_space("as")?;
                self.ty(ty)?;
                self.tail(tail)?;
                Ok(())
            })
            .otherwise(|| {
                self.has_vertical_shape(VerticalShape::HangingIndent, || {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("as")?;
                        self.ty(ty)?;
                        self.tail(tail)?;
                        Ok(())
                    })
                })
            })?;
        Ok(())
    }

    fn paren(&self, inner: &ast::Expr, tail: &Tail) -> FormatResult {
        self.out.token("(")?;
        self.backtrack()
            .next(|| {
                self.with_vertical_shape_min(VerticalShape::List, || self.expr(inner))?;
                self.out.token(")")?;
                self.tail(tail)?;
                Ok(())
            })
            .otherwise(|| {
                self.enclosed_after_opening(")", || self.expr(inner))?;
                self.tail(tail)?;
                Ok(())
            })?;
        Ok(())
    }

    pub fn range(
        &self,
        start: Option<&ast::Expr>,
        sigil: &'static str,
        end: Option<&ast::Expr>,
        tail: &Tail,
    ) -> FormatResult {
        if let Some(start) = start {
            let first_line = self.out.line();
            self.expr_tail(
                start,
                &self.tail_fn(|af| {
                    af.out.token(sigil)?;
                    let Some(end) = end else {
                        return af.tail(tail);
                    };
                    if af.out.line() == first_line {
                        af.expr_tail(end, tail)?;
                    } else {
                        self.has_vertical_shape(VerticalShape::Unrestricted, || {
                            af.expr_tail(end, tail)
                        })?;
                    }
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
        self.expr_tail(func, &self.tail_token("("))?;
        self.has_vertical_shape_if(
            self.out.line() != first_line,
            VerticalShape::Unrestricted,
            || self.call_args_after_open_paren(args, tail),
        )?;
        Ok(())
    }

    pub fn call_args_after_open_paren(&self, args: &[P<ast::Expr>], tail: &Tail) -> FormatResult {
        let mut list_opt = expr_list_opt();
        let is_only_closure = args.len() == 1 && matches!(args[0].kind, ast::ExprKind::Closure(_));
        if !is_only_closure {
            list_opt = list_opt
                .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.fn_call_width);
        }
        self.list(
            Braces::Parens,
            args,
            self.expr_list_item(),
            list_opt.omit_open_brace().tail(tail),
        )
    }

    fn delim_args(&self, delim_args: &ast::DelimArgs) -> FormatResult {
        if matches!(delim_args.delim, rustc_ast::token::Delimiter::Brace) {
            self.out.space()?;
        }
        self.out.copy_span(delim_args.dspan.entire())
    }

    fn if_<'a>(
        &self,
        condition: &ast::Expr,
        block: &'a ast::Block,
        else_: Option<&'a ast::Expr>,
        tail: &Tail,
    ) -> FormatResult {
        let start_pos = self.out.last_line_len();
        let is_head_single_line = self.token_expr_open_brace("if", condition)?;

        let single_line = (|| {
            if !is_head_single_line {
                return None;
            }
            let else_ = else_?;
            let ast::ExprKind::Block(else_block, _) = &else_.kind else {
                return None;
            };
            let block_expr = self.try_into_expr_only_block(block)?;
            let else_expr = self.try_into_expr_only_block(else_block)?;

            Some(move || {
                self.with_single_line(|| {
                    self.with_width_limit_from_start(
                        start_pos,
                        RUSTFMT_CONFIG_DEFAULTS.single_line_if_else_max_width,
                        || {
                            self.expr_only_block_after_open_brace(block_expr)?;
                            self.out.space_token_space("else")?;
                            self.expr_only_block(else_expr)?;
                            self.tail(tail)?;
                            Ok(())
                        },
                    )
                })
            })
        })();

        let multi_line = || {
            // todo this is failing earlier than "indent middle" is really violated;
            //   do we need to revise the guidelines in VerticalShape docs?
            self.has_vertical_shape_if(else_.is_some(), VerticalShape::Unrestricted, || {
                self.block_expr(true, block)
            })?;
            let mut else_ = else_;
            loop {
                let Some(else_expr) = else_ else { break };
                self.out.space_token_space("else")?;
                match &else_expr.kind {
                    ast::ExprKind::Block(block, _) => {
                        self.block_expr(false, block)?;
                        break;
                    }
                    ast::ExprKind::If(condition, next_block, next_else) => {
                        self.token_expr_open_brace("if", condition)?;
                        self.block_expr(true, next_block)?;
                        else_ = next_else.as_deref();
                    }
                    _ => unreachable!(),
                }
            }
            self.tail(tail)?;
            Ok(())
        };

        self.backtrack().next_opt(single_line).otherwise(multi_line)
    }

    pub fn token_expr_open_brace(
        &self,
        token: &'static str,
        expr: &ast::Expr,
    ) -> FormatResult<bool> {
        self.has_vertical_shape(VerticalShape::Unrestricted, || {
            let first_line = self.out.line();
            self.out.token_space(token)?;
            self.expr(expr)?;
            self.backtrack()
                .next_if(
                    self.out.line() == first_line || self.out.last_line_is_closers(),
                    || self.with_single_line(|| self.out.space_token("{")),
                )
                .otherwise(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    self.out.token("{")?;
                    Ok(())
                })?;
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
        self.list(
            Braces::Curly,
            &struct_.fields,
            Self::expr_field,
            list_opt()
                // todo not wide enough?
                .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
                .rest(ListRest::from_struct_rest(&struct_.rest))
                .tail(tail),
        )?;
        Ok(())
    }

    fn expr_field(
        &self,
        field: &ast::ExprField,
        tail: &Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        self.with_attrs_tail(&field.attrs, field.span, tail, || {
            self.ident(field.ident)?;
            if field.is_shorthand {
                self.tail(tail)?;
            } else {
                self.out.token_space(":")?;
                self.expr_tail(&field.expr, tail)?;
            }
            Ok(())
        })
    }

    pub fn while_(&self, condition: &ast::Expr, block: &ast::Block) -> FormatResult {
        self.token_expr_open_brace("while", condition)?;
        self.block_expr(true, block)?;
        Ok(())
    }
}

pub fn expr_list_opt<'ast, 'tail>() -> ListOptions<'ast, 'tail, P<ast::Expr>> {
    list_opt::<P<ast::Expr>>()
        .item_prefers_overflow(|expr| matches!(expr.kind, ast::ExprKind::Closure(_)))
}
