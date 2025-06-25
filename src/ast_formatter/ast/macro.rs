use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies, VerticalListStrategy,
    WrapToFit,
};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::macro_args::{MacroArgs, mac_call_id};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use std::num::NonZero;

impl AstFormatter {
    pub fn macro_call(&self, mac_call: &ast::MacCall, tail: Tail) -> FormatResult {
        self.path(&mac_call.path, true)?;
        self.out.token("!")?;
        let args = self.module.macro_args.get(&mac_call_id(mac_call));
        if let Some(args) = args {
            self.macro_args(args, tail)?;
        } else {
            if matches!(mac_call.args.delim, rustc_ast::token::Delimiter::Brace) {
                self.out.space()?;
            }
            self.out.copy_span(mac_call.args.dspan.entire())?;
            self.tail(tail)?;
        }
        Ok(())
    }

    fn macro_args(&self, args: &MacroArgs, tail: Tail) -> FormatResult {
        match *args {
            MacroArgs::Cfg(ref args) => self.macro_args_list(args, None, tail, |af, item, tail| {
                af.meta_item_inner(item)?;
                af.tail(tail)?;
                Ok(())
            })?,
            MacroArgs::FnLike(ref args) => {
                self.macro_args_list(args, None, tail, |af, expr, tail| af.expr_tail(expr, tail))?
            }
            MacroArgs::Format {
                ref args,
                format_string_pos,
            } => self.macro_args_list(args, Some(format_string_pos), tail, |af, item, tail| {
                af.expr_tail(item, tail)
            })?,
            MacroArgs::Matches(ref expr, ref pat, ref guard) => {
                self.matches_args(expr, pat, guard.as_deref(), tail)?
            }
        }
        Ok(())
    }

    fn macro_args_list<T>(
        &self,
        args: &[T],
        format_string_pos: Option<u8>,
        tail: Tail,
        format: impl Fn(&Self, &T, Tail) -> FormatResult,
    ) -> FormatResult {
        self.list(
            Braces::Parens,
            &args,
            |af, item, tail, _lcx| format(af, item, tail),
            ListOptions {
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
                    horizontal: HorizontalListStrategy {
                        contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                        ..
                    },
                    vertical: VerticalListStrategy {
                        wrap_to_fit: format_string_pos.map(|format_string_pos| {
                            WrapToFit {
                                format_string_pos: Some(format_string_pos),
                                // todo rename/consolidate this variable
                                max_element_width: Some(
                                    NonZero::new(
                                        RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold,
                                    )
                                    .unwrap(),
                                ),
                            }
                        }),
                        ..
                    },
                    ..
                }),
                tail,
                ..
            },
        )
    }

    fn matches_args(
        &self,
        expr: &ast::Expr,
        pat: &ast::Pat,
        guard: Option<&ast::Expr>,
        tail: Tail,
    ) -> FormatResult {
        self.out.token("(")?;
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    self.with_width_limit(RUSTFMT_CONFIG_DEFAULTS.fn_call_width, || {
                        self.expr(expr)?;
                        self.out.token_space(",")?;
                        self.pat(pat)?;
                        if let Some(guard) = guard {
                            self.out.space_token_space("if")?;
                            self.expr(guard)?;
                        }
                        Ok(())
                    })?;
                    self.out.token_skip_if_present(",")?;
                    self.out.token(")")?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|| {
                self.indented(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    self.expr(expr)?;
                    self.out.token(",")?;
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    let (pat_first_line, pat_start) = self.out.line_col();
                    self.pat(pat)?;
                    let pat_width = (self.out.line() == pat_first_line)
                        .then(|| self.out.col() - pat_start);
                    // todo width limit on pat + guard single line?
                    if let Some(guard) = guard {
                        // todo introduce constant
                        let allow_same_line = pat_width.is_some_and(|w| w <= 40);
                        self.backtrack()
                            .next_if(allow_same_line, || {
                                self.could_wrap_indent(|| {
                                    self.out.space_token_space("if")?;
                                    self.expr(guard)?;
                                    self.out.token_maybe_missing(",")?;
                                    Ok(())
                                })
                            })
                            .next(|| {
                                self.indented(|| {
                                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                                    self.out.token_space("if")?;
                                    self.expr(guard)?;
                                    self.out.token_maybe_missing(",")?;
                                    Ok(())
                                })
                            })
                            .result()?;
                    } else {
                        self.out.token_maybe_missing(",")?;
                    }
                    Ok(())
                })?;
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                self.out.token(")")?;
                self.tail(tail)?;
                Ok(())
            })
            .result()
    }
}
