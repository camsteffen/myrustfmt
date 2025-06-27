mod binary_expr;
mod r#match;
mod postfix;

use crate::ast_formatter::ast::r#macro::MacCallSemi;
use crate::ast_formatter::brackets::Brackets;
use crate::ast_formatter::list::ListItemContext;
use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies, VerticalListStrategy,
};
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::debug::expr_kind_name;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_utils::{is_jump_expr, plain_block, postfix_expr_kind};
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use tracing::instrument;

impl AstFormatter {
    pub fn expr(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, None)
    }

    pub fn expr_tail(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        self.with_attrs_tail(&expr.attrs, expr.span.into(), tail, || {
            self.expr_after_attrs(expr, tail)
        })
    }

    #[instrument(name = "expr", skip_all, fields(kind=expr_kind_name(expr)))]
    fn expr_after_attrs(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let mut tail_opt = Some(tail);
        let mut take_tail = || tail_opt.take().unwrap();
        match expr.kind {
            postfix_expr_kind!() => self.postfix_chain(expr, take_tail())?,
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => {
                self.addr_of(borrow_kind, mutability, target, take_tail())?
            }
            ast::ExprKind::Array(ref items) => self.array(items, take_tail())?,
            ast::ExprKind::Assign(ref left, ref right, _) => {
                self.expr_infix(left, "=", right, take_tail())?
            }
            ast::ExprKind::AssignOp(op, ref left, ref right) => {
                self.expr_infix(left, op.node.as_str(), right, take_tail())?
            }
            ast::ExprKind::Binary(op, ref left, ref right) => {
                self.binary_expr(left, right, op, take_tail())?
            }
            ast::ExprKind::Block(ref block, label) => {
                self.block_expr_allow_horizontal(label, block, take_tail())?
            }
            ast::ExprKind::Break(label, ref inner) => {
                self.break_(label, inner.as_deref(), take_tail())?
            }
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args, take_tail())?,
            ast::ExprKind::Cast(ref target, ref ty) => self.cast(target, ty, take_tail())?,
            ast::ExprKind::Closure(ref closure) => self.closure(closure, take_tail())?,
            ast::ExprKind::ConstBlock(ref anon_const) => {
                self.out.token_space("const")?;
                self.expr_tail(&anon_const.value, take_tail())?;
            }
            ast::ExprKind::Continue(label) => self.continue_(label)?,
            ast::ExprKind::ForLoop {
                ref pat,
                ref iter,
                ref body,
                label,
                ..
            } => self.for_loop(pat, iter, body, label)?,
            ast::ExprKind::If(ref condition, ref block, ref else_) => {
                self.if_(condition, block, else_.as_deref(), take_tail())?
            }
            ast::ExprKind::Let(ref pat, ref init, ..) => self.let_(pat, init, take_tail())?,
            ast::ExprKind::Lit(_) => self.out.copy_span(expr.span.into())?,
            ast::ExprKind::Loop(ref block, label, _) => self.loop_(label, block)?,
            ast::ExprKind::MacCall(ref mac_call) => {
                self.macro_call(mac_call, MacCallSemi::No, take_tail())?
            }
            ast::ExprKind::Match(ref scrutinee, ref arms, ast::MatchKind::Prefix) => {
                self.match_(scrutinee, arms)?
            }
            ast::ExprKind::Paren(ref inner) => self.paren(inner, take_tail())?,
            ast::ExprKind::Path(ref qself, ref path) => self.qpath(qself, path, true, take_tail())?,
            ast::ExprKind::Range(ref start, ref end, limits) => self.range(
                start.as_deref(),
                limits.as_str(),
                end.as_deref(),
                take_tail(),
            )?,
            ast::ExprKind::Repeat(ref element, ref count) => {
                self.repeat(element, count, take_tail())?
            }
            ast::ExprKind::Ret(ref target) => self.return_(target.as_deref(), take_tail())?,
            ast::ExprKind::Struct(ref struct_) => self.struct_expr(struct_, take_tail())?,
            ast::ExprKind::Tup(ref items) => self.tuple(items, take_tail())?,
            ast::ExprKind::Unary(op, ref target) => {
                self.out.token(op.as_str())?;
                self.expr_tail(target, take_tail())?;
            }
            ast::ExprKind::Underscore => self.out.token("_")?,
            ast::ExprKind::While(ref condition, ref block, _label) => {
                self.while_(condition, block)?
            }
            ast::ExprKind::Become(_)
            // todo
            | ast::ExprKind::FormatArgs(_)
            | ast::ExprKind::Gen(..)
            | ast::ExprKind::InlineAsm(_)
            | ast::ExprKind::Match(.., ast::MatchKind::Postfix)
            | ast::ExprKind::TryBlock(_)
            | ast::ExprKind::Type(..)
            | ast::ExprKind::UnsafeBinderCast(..)
            | ast::ExprKind::Use(..)
            | ast::ExprKind::Yeet(_)
            | ast::ExprKind::Yield(_) => return Err(FormatErrorKind::UnsupportedSyntax.into()),
            ast::ExprKind::Dummy
            | ast::ExprKind::Err(_)
            | ast::ExprKind::IncludedBytes(_)
            | ast::ExprKind::OffsetOf(..) => panic!("unexpected ExprKind"),
        }
        if let Some(tail) = tail_opt {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn addr_of(
        &self,
        borrow_kind: ast::BorrowKind,
        mutability: ast::Mutability,
        expr: &ast::Expr,
        tail: Tail,
    ) -> FormatResult {
        match borrow_kind {
            ast::BorrowKind::Raw => {
                self.out.token_space("&raw")?;
                match mutability {
                    ast::Mutability::Mut => self.out.token_space("mut")?,
                    ast::Mutability::Not => self.out.token_space("const")?,
                }
            }
            ast::BorrowKind::Ref => {
                self.out.token("&")?;
                self.mutability(mutability)?;
            }
        }
        self.expr_tail(expr, tail)?;
        Ok(())
    }

    fn array(&self, items: &[P<ast::Expr>], tail: Tail) -> FormatResult {
        self.list(
            Brackets::Square,
            items,
            |af, expr, tail, _lcx| af.expr_tail(expr, tail),
            ListOptions {
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
                    horizontal: HorizontalListStrategy {
                        contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.array_width),
                        ..
                    },
                    vertical: VerticalListStrategy::wrap_to_fit(Some(
                        RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold,
                    )),
                    ..
                }),
                tail,
                ..
            },
        )
    }

    fn break_(
        &self,
        label: Option<ast::Label>,
        expr: Option<&ast::Expr>,
        tail: Tail,
    ) -> FormatResult {
        self.out.token("break")?;
        if let Some(label) = label {
            self.out.space()?;
            self.label(label)?;
        }
        let Some(expr) = expr else {
            return self.tail(tail);
        };
        self.out.space()?;
        self.expr_tail(expr, tail)?;
        Ok(())
    }

    pub fn call(&self, func: &ast::Expr, args: &[P<ast::Expr>], tail: Tail) -> FormatResult {
        let first_line = self.out.line();
        self.expr_tail(func, Some(&self.tail_token("(")))?;
        self.has_vstruct_if(self.out.line() > first_line, VStruct::NonBlockIndent, || {
            self.call_args(args, ListStrategies::flexible_overflow(), tail)
        })?;
        Ok(())
    }

    pub fn call_args(
        &self,
        args: &[P<ast::Expr>],
        mut list_strategies: ListStrategies<P<ast::Expr>>,
        tail: Tail,
    ) -> FormatResult {
        if let Some(horizontal) = list_strategies.get_horizontal_mut() {
            horizontal.contents_max_width = Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width);
        }
        self.list(
            Brackets::Parens,
            args,
            |af, expr, tail, lcx| {
                if !lcx.is_vertical && lcx.index == args.len() - 1 {
                    let mut vstructs =
                        VStruct::ControlFlow | VStruct::Index | VStruct::NonBlockIndent;
                    if args.len() > 1 {
                        // todo maybe just look for closure explicitly?
                        // todo or can we collapse some of these variants?
                        // really it's anything that isn't a closure
                        vstructs |= VStruct::Block | VStruct::List | VStruct::Match;
                    }
                    af.disallow_vstructs(vstructs, || af.expr_tail(expr, tail))?;
                } else {
                    af.expr_tail(expr, tail)?;
                }
                Ok(())
            },
            ListOptions {
                omit_open_bracket: true,
                strategies: list_strategies,
                tail,
                ..
            },
        )?;
        Ok(())
    }

    fn cast(&self, target: &ast::Expr, ty: &ast::Ty, tail: Tail) -> FormatResult {
        self.expr(target)?;
        self.backtrack()
            .next(|| {
                self.space_could_wrap_indent(|| {
                    self.out.token_space("as")?;
                    self.ty(ty)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|| {
                self.has_vstruct(VStruct::NonBlockIndent, || {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("as")?;
                        self.ty(ty)?;
                        self.tail(tail)?;
                        Ok(())
                    })
                })
            })
            .result()?;
        Ok(())
    }

    fn continue_(&self, label: Option<ast::Label>) -> FormatResult {
        self.out.token("continue")?;
        if let Some(label) = label {
            self.out.space()?;
            self.label(label)?;
        }
        Ok(())
    }

    fn for_loop(
        &self,
        pat: &ast::Pat,
        iter: &ast::Expr,
        body: &ast::Block,
        label: Option<ast::Label>,
    ) -> FormatResult {
        self.has_vstruct(VStruct::ControlFlow, || {
            self.label_colon(label)?;
            self.out.token_space("for")?;
            self.pat(pat)?;
            // todo comments
            let wrapped_iter = self
                .backtrack()
                .next(|| {
                    self.could_wrap_indent(|| {
                        self.out.space_token_space("in")?;
                        self.expr_tail(
                            iter,
                            Some(&self.tail_fn(|af| {
                                af.out.space()?;
                                af.out.token("{")?;
                                Ok(())
                            })),
                        )?;
                        Ok(false)
                    })
                })
                .next(|| {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("in")?;
                        self.expr(iter)?;
                        Ok(())
                    })?;
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    Ok(true)
                })
                .result()?;
            self.block_expr(!wrapped_iter, body)?;
            Ok(())
        })
    }

    fn if_<'a>(
        &self,
        condition: &ast::Expr,
        block: &'a ast::Block,
        else_: Option<&'a ast::Expr>,
        tail: Tail,
    ) -> FormatResult {
        self.has_vstruct(VStruct::ControlFlow, || {
            let (first_line, start_col) = self.out.line_col();
            self.control_flow_header("if", condition)?;

            let single_line = (|| {
                if self.out.line() != first_line {
                    return None;
                }
                let else_ = else_?;
                let ast::ExprKind::Block(else_block, _) = &else_.kind else {
                    return None;
                };
                let block_expr = self.try_into_optional_block(block)?;
                let else_expr = self.try_into_optional_block(else_block)?;

                Some(move || {
                    self.with_single_line(|| {
                        self.with_width_limit_end(
                            start_col + RUSTFMT_CONFIG_DEFAULTS.single_line_if_else_max_width,
                            || {
                                self.optional_block_horizontal_after_open_brace(block_expr)?;
                                self.out.space_token_space("else")?;
                                self.optional_block_horizontal(else_expr)?;
                                self.tail(tail)?;
                                Ok(())
                            },
                        )
                    })
                })
            })();

            let multi_line = || {
                self.block_expr(true, block)?;
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
                            self.control_flow_header("if", condition)?;
                            self.block_expr(true, next_block)?;
                            else_ = next_else.as_deref();
                        }
                        _ => unreachable!(),
                    }
                }
                self.tail(tail)?;
                Ok(())
            };

            self.backtrack()
                .next_opt(single_line)
                .next(multi_line)
                .result()
        })
    }

    pub fn label(&self, label: ast::Label) -> FormatResult {
        self.ident(label.ident)
    }

    pub fn label_colon(&self, label: Option<ast::Label>) -> FormatResult {
        if let Some(label) = label {
            self.label(label)?;
            self.out.token(":")?;
            self.out.space()?;
        }
        Ok(())
    }

    fn let_(&self, pat: &ast::Pat, init: &ast::Expr, tail: Tail) -> FormatResult {
        self.out.token_space("let")?;
        self.pat(pat)?;
        self.out.space_token_space("=")?;
        self.expr_tail(init, tail)?;
        Ok(())
    }

    fn loop_(&self, label: Option<ast::Label>, block: &ast::Block) -> FormatResult {
        self.has_vstruct(VStruct::ControlFlow, || {
            self.label_colon(label)?;
            self.out.token_space("loop")?;
            self.block_expr(false, block)?;
            Ok(())
        })
    }

    fn paren(&self, inner: &ast::Expr, tail: Tail) -> FormatResult {
        self.out.token("(")?;
        self.backtrack()
            .next(|| {
                self.disallow_vstructs(VStruct::NonBlockIndent, || {
                    let expr_start = self.out.col();
                    self.expr_tail(
                        inner,
                        Some(&self.tail_fn(|af| {
                            let end_start = self.out.col();
                            let before_end = self.out.checkpoint();
                            let Err(err) = self.out.with_recover_width(|| -> FormatResult {
                                af.out.token(")")?;
                                af.tail(tail)?;
                                Ok(())
                            }) else {
                                return Ok(());
                            };
                            if err.kind != FormatErrorKind::WidthLimitExceeded {
                                return Err(err);
                            }
                            let expr_width = end_start - expr_start;
                            let end_end = self.out.col();
                            let end_width = end_end - end_start;
                            let next_inside_end = INDENT_WIDTH + expr_width;
                            if next_inside_end.max(end_width)
                                < end_end - self.out.total_indent.get()
                            {
                                // multi-line strategy
                                return Err(err);
                            }
                            self.out.restore_checkpoint(&before_end);
                            af.out.token(")")?;
                            self.tail(tail)?;
                            Ok(())
                        })),
                    )
                })
            })
            .next(|| {
                self.enclosed_contents(|| self.expr(inner))?;
                self.out.token(")")?;
                self.tail(tail)?;
                Ok(())
            })
            .result()?;
        Ok(())
    }

    pub fn range(
        &self,
        start: Option<&ast::Expr>,
        sigil: &'static str,
        end: Option<&ast::Expr>,
        tail: Tail,
    ) -> FormatResult {
        if let Some(start) = start {
            let first_line = self.out.line();
            self.expr_tail(
                start,
                Some(&self.tail_fn(|af| {
                    af.out.token(sigil)?;
                    let Some(end) = end else { return af.tail(tail) };
                    self.has_vstruct_if(af.out.line() > first_line, VStruct::NonBlockIndent, || {
                        af.expr_tail(end, tail)
                    })?;
                    Ok(())
                })),
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

    fn repeat(&self, element: &ast::Expr, count: &ast::AnonConst, tail: Tail) -> FormatResult {
        self.out.token("[")?;
        self.expr_tail(
            element,
            Some(&self.tail_fn(|af| {
                af.out.token_space(";")?;
                af.expr(&count.value)?;
                af.out.token("]")?;
                self.tail(tail)?;
                Ok(())
            })),
        )?;
        Ok(())
    }

    fn return_(&self, target: Option<&ast::Expr>, tail: Tail) -> FormatResult {
        self.out.token("return")?;
        let Some(target) = target else {
            return self.tail(tail);
        };
        self.out.space()?;
        self.expr_tail(target, tail)?;
        Ok(())
    }

    fn struct_expr(&self, struct_: &ast::StructExpr, tail: Tail) -> FormatResult {
        let first_line = self.out.line();
        self.qpath(&struct_.qself, &struct_.path, true, None)?;
        self.has_vstruct_if(self.out.line() > first_line, VStruct::NonBlockIndent, || {
            self.out.space()?;
            self.list(
                Brackets::Curly,
                &struct_.fields,
                Self::struct_field,
                ListOptions {
                    is_struct: true,
                    rest: ListRest::from_struct_rest(&struct_.rest),
                    strategies: if self.out.line() > first_line {
                        ListStrategies::vertical()
                    } else {
                        ListStrategies::Flexible(FlexibleListStrategy {
                            horizontal: HorizontalListStrategy {
                                // todo not wide enough?
                                contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width),
                                ..
                            },
                            ..
                        })
                    },
                    tail,
                    ..
                },
            )?;
            Ok(())
        })?;
        Ok(())
    }

    fn struct_field(
        &self,
        field: &ast::ExprField,
        tail: Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        self.with_attrs_tail(&field.attrs, field.span.into(), tail, || {
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

    fn tuple(&self, items: &[P<ast::Expr>], tail: Tail) -> FormatResult {
        self.list(
            Brackets::Parens,
            items,
            |af, expr, tail, _lcx| af.expr_tail(expr, tail),
            ListOptions {
                force_trailing_comma: items.len() == 1,
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
                    horizontal: HorizontalListStrategy {
                        contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                        ..
                    },
                    ..
                }),
                tail,
                ..
            },
        )
    }

    pub fn while_(&self, condition: &ast::Expr, block: &ast::Block) -> FormatResult {
        self.has_vstruct(VStruct::ControlFlow, || {
            self.control_flow_header("while", condition)?;
            self.block_expr(true, block)?;
            Ok(())
        })
    }
}

// helpers/utils
impl AstFormatter {
    pub fn control_flow_header(&self, keyword: &'static str, expr: &ast::Expr) -> FormatResult {
        self.has_vstruct(VStruct::NonBlockIndent, || {
            let first_line = self.out.line();
            self.out.token_space(keyword)?;
            self.expr(expr)?;
            self.backtrack()
                .next_if(
                    self.out.line() == first_line || self.out.last_line_is_closers(),
                    || self.with_single_line(|| self.out.space_token("{")),
                )
                .next(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    self.out.token("{")?;
                    Ok(())
                })
                .result()?;
            Ok(())
        })
    }

    pub fn expr_force_plain_block(&self, expr: &ast::Expr) -> FormatResult {
        match plain_block(expr) {
            Some(block) => self.block_expr(false, block),
            None => self.add_block(|| self.expr_stmt(expr)),
        }
    }

    pub fn expr_stmt(&self, expr: &ast::Expr) -> FormatResult {
        self.expr_tail(expr, Some(&self.tail_fn(|af| af.expr_stmt_semi(expr))))
    }

    pub fn expr_stmt_semi(&self, expr: &ast::Expr) -> FormatResult {
        if is_jump_expr(expr) {
            self.out.token_insert(";")?;
        }
        Ok(())
    }
}
