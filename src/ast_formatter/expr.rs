use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemConfig, ListItemContext, ListStrategy};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::builder::{list, FormatListItem, ListBuilder};
use crate::ast_formatter::list::list_config::{ArrayListConfig, TupleListConfig, ListConfig};
use crate::ast_utils::postfix_expr_kind;
use crate::constraints::MultiLineShape;
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Tail::none())
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        self.with_attrs_tail(&expr.attrs, expr.span, tail, || {
            self.expr_after_attrs(expr, tail)
        })
    }

    pub fn expr_after_attrs(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();
        match expr.kind {
            ast::ExprKind::Array(ref items) => {
                self.expr_list(Braces::SQUARE, items)
                    .config(ArrayListConfig)
                    .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.array_width)
                    .tail(take_tail())
                    .format(self)?
            }
            ast::ExprKind::ConstBlock(ref anon_const) => {
                self.out.token_space("const")?;
                self.anon_const_tail(anon_const, take_tail())?;
            }
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, take_tail())?,
            postfix_expr_kind!() => self.postfix_chain(expr, take_tail())?,
            ast::ExprKind::Tup(ref items) => {
                self.expr_list(Braces::PARENS, items)
                    .config(TupleListConfig { len: items.len() })
                    .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
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
                let tail = take_tail();
                self.expr(target)?;
                self.backtrack()
                    .next(|| {
                        self.out.space_token_space("as")?;
                        self.ty(ty)?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .otherwise(|| {
                        self.constraints()
                            .with_single_line_unless(MultiLineShape::HangingIndent, || {
                                self.indented(|| {
                                    self.out.newline_within_indent()?;
                                    self.out.token_space("as")?;
                                    self.ty(ty)?;
                                    self.tail(tail)?;
                                    Ok(())
                                })
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
                self.block_separate_lines(body)?;
            }
            ast::ExprKind::Loop(ref block, label, _) => {
                self.label(label)?;
                self.out.token_space("loop")?;
                self.block_separate_lines(block)?;
            }
            ast::ExprKind::Match(ref scrutinee, ref arms, match_kind) => match match_kind {
                ast::MatchKind::Postfix => todo!(),
                ast::MatchKind::Prefix => self.match_(scrutinee, arms)?,
            },
            ast::ExprKind::Closure(ref closure) => self.closure(closure, take_tail())?,
            ast::ExprKind::Block(ref block, label) => self.block_expr(label, block, take_tail())?,
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
                        self.constraints()
                            .with_multi_line_shape_min(MultiLineShape::VerticalList, || {
                                self.expr(inner)
                            })?;
                        self.out.token(")")?;
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
            ast::ExprKind::UnsafeBinderCast(..) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
        if let Some(tail) = tail {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn expr_list<'ast>(
        &self,
        braces: &'static Braces,
        expr_list: &'ast [P<ast::Expr>],
    ) -> ListBuilder<
        'ast,
        '_,
        P<ast::Expr>,
        impl FormatListItem<P<ast::Expr>>,
        impl ListConfig,
        impl ListItemConfig<Item = P<ast::Expr>>,
    > {
        struct ExprListItemConfig;
        impl ListItemConfig for ExprListItemConfig {
            type Item = P<ast::Expr>;

            fn last_item_prefers_overflow(expr: &Self::Item) -> bool {
                matches!(expr.kind, ast::ExprKind::Closure(_))
            }
        }
        list(braces, expr_list, self.expr_list_item())
            .item_config(ExprListItemConfig)
    }

    pub fn expr_list_item(
        &self,
    ) -> impl Fn(&AstFormatter, &P<ast::Expr>, &Tail, ListItemContext) -> FormatResult {
        // todo kinda hacky
        let outer_multi_line = self.constraints().borrow().multi_line;

        move |af, expr, tail, lcx| {
            af.skip_single_expr_blocks_tail(expr, tail, |expr, tail| {
                let format = || af.expr_tail(expr, tail);
                match lcx.strategy {
                    // overflow last item
                    ListStrategy::SingleLine
                        if outer_multi_line >= MultiLineShape::VerticalList
                            && lcx.index == lcx.len - 1 =>
                    {
                        // override the multi-line shape to be less strict than SingleLine
                        let shape = if lcx.len > 1 {
                            // don't overflow nested lists in a list with multiple items
                            MultiLineShape::BlockIndent
                        } else {
                            MultiLineShape::VerticalList
                        };
                        // todo avoid replace?
                        af.constraints()
                            .with_multi_line_shape_replaced(shape, format)?;
                        Ok(())
                    }
                    // on separate lines, enforce IndentMiddle by adding a block
                    ListStrategy::SeparateLines if lcx.len > 1 => {
                        af.backtrack()
                            .next(|| {
                                af.constraints().with_multi_line_shape_min(
                                    MultiLineShape::HangingIndent,
                                    format,
                                )
                            })
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

    pub fn label(&self, label: Option<ast::Label>) -> FormatResult {
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
                &self.tail_fn(|af| {
                    af.out.token(sigil)?;
                    let Some(end) = end else {
                        return af.tail(tail);
                    };
                    if af.out.line() == first_line {
                        af.expr_tail(end, tail)?;
                    } else {
                        self.constraints()
                            .with_single_line_unless(MultiLineShape::DisjointIndent, || {
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
        let is_multi_line_func = self.out.line() != first_line;
        self.constraints().with_single_line_unless_opt(
            is_multi_line_func.then_some(MultiLineShape::DisjointIndent),
            || self.call_args_after_open_paren(args, tail),
        )?;
        Ok(())
    }

    pub fn call_args_after_open_paren(&self, args: &[P<ast::Expr>], tail: &Tail) -> FormatResult {
        let mut list = self.expr_list(Braces::PARENS, args);
        let width_limit_applies = match args {
            [arg] => !matches!(arg.kind, ast::ExprKind::Closure(_)),
            _ => true,
        };
        if width_limit_applies {
            list = list.single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.fn_call_width);
        }
        list.omit_open_brace().tail(tail).format(self)
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
            //   do we need to revise the guidelines in MultiLineConstraint docs?
            self.constraints().with_single_line_unless_opt(
                else_.is_some().then_some(MultiLineShape::DisjointIndent),
                || self.block_separate_lines_after_open_brace(block),
            )?;
            let mut else_ = else_;
            loop {
                let Some(else_expr) = else_ else { break };
                self.out.space_token_space("else")?;
                match &else_expr.kind {
                    ast::ExprKind::Block(block, _) => {
                        self.block_separate_lines(block)?;
                        break;
                    }
                    ast::ExprKind::If(condition, next_block, next_else) => {
                        self.token_expr_open_brace("if", condition)?;
                        self.block_separate_lines_after_open_brace(next_block)?;
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

    pub fn token_expr_open_brace(&self, token: &str, expr: &ast::Expr) -> FormatResult<bool> {
        self.constraints()
            .with_single_line_unless(MultiLineShape::DisjointIndent, || {
                let first_line = self.out.line();
                self.out.token_space(token)?;
                self.expr(expr)?;
                self.backtrack()
                    .next_if(
                        self.out.line() == first_line || self.out.last_line_is_closers(),
                        || self.with_single_line(|| self.out.space_token("{")),
                    )
                    .otherwise(|| {
                        self.out.newline_within_indent()?;
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
        list(Braces::CURLY, &struct_.fields, Self::expr_field)
            // todo not wide enough?
            .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
            .rest(ListRest::from(&struct_.rest))
            .tail(tail)
            .format(self)?;
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
        self.block_separate_lines_after_open_brace(block)?;
        Ok(())
    }
}
