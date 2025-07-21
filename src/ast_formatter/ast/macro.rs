use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::brackets::Brackets;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies, VerticalListStrategy,
    WrapToFit,
};
use crate::ast_formatter::std_macro::std_macro;
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::width_thresholds::WIDTH_THRESHOLDS;
use crate::error::FormatResult;
use crate::macro_args::{MacroArgs, mac_call_id};
use crate::span::Span;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::token::Delimiter;
use std::num::NonZero;

#[derive(Clone, Copy)]
pub enum MacCallSemi {
    Yes,
    No,
    Item,
}

impl AstFormatter {
    pub fn macro_call(
        &self,
        mac_call: &ast::MacCall,
        semi: MacCallSemi,
        tail: Tail,
    ) -> FormatResult {
        self.path(&mac_call.path, true)?;
        self.out.token("!")?;
        let mac_args = self.module.macro_args.get(&mac_call_id(mac_call));
        let brackets = if let Some(std_macro) = std_macro(mac_call) {
            std_macro.brackets()
        } else {
            match mac_call.args.delim {
                Delimiter::Brace => Brackets::Curly,
                Delimiter::Bracket => Brackets::Square,
                Delimiter::Parenthesis => Brackets::Parens,
                Delimiter::Invisible(_) => panic!("unexpected Invisible delimiter"),
            }
        };
        if brackets == Brackets::Curly {
            self.out.space()?;
        }
        let tail = |af: &Self| {
            af.macro_call_semi(semi, brackets)?;
            af.tail(tail)?;
            Ok(())
        };
        self.out.token_replace(brackets.start())?;
        if mac_call.args.tokens.is_empty() {
            self.enclosed_empty_contents()?;
            self.out.token_replace(brackets.end())?;
            tail(self)?;
        } else if let Some(args) = mac_args {
            self.macro_args(brackets, args, Some(&self.tail_fn(tail)))?;
        } else {
            let dspan = mac_call.args.dspan;
            // skip open bracket but include closing bracket
            let span = Span {
                lo: dspan.open.hi(),
                hi: dspan.close.hi(),
            };
            self.out.copy_span(span)?;
            tail(self)?;
        }
        Ok(())
    }

    fn macro_args(&self, brackets: Brackets, args: &MacroArgs, tail: Tail) -> FormatResult {
        match *args {
            MacroArgs::Cfg(ref args) => {
                self.macro_args_list(brackets, args, None, tail, |af, item, tail| {
                    af.meta_item_inner(item)?;
                    af.tail(tail)?;
                    Ok(())
                })?
            }
            MacroArgs::FnLike(ref args) => {
                self.macro_args_list(brackets, args, None, tail, |af, expr, tail| {
                    af.expr_tail(expr, tail)
                })?
            }
            MacroArgs::Format {
                ref args,
                format_string_pos,
            } => self.macro_args_list(
                brackets,
                args,
                Some(format_string_pos),
                tail,
                |af, item, tail| af.expr_tail(item, tail),
            )?,
            MacroArgs::Matches(ref expr, ref pat, ref guard) => {
                self.matches_args(expr, pat, guard.as_deref(), tail)?
            }
            MacroArgs::ThreadLocal(ref stmts) => {
                self.enclosed_contents(|| {
                    self.list_with_item_sorting(stmts, |stmt| self.stmt(stmt))
                })?;
                self.out.token_replace(brackets.end())?;
                self.out.token_skip_if_present(";")?;
                self.tail(tail)?;
            }
        }
        Ok(())
    }

    fn macro_args_list<T>(
        &self,
        brackets: Brackets,
        args: &[T],
        format_string_pos: Option<u8>,
        tail: Tail,
        format: impl Fn(&Self, &T, Tail) -> FormatResult,
    ) -> FormatResult {
        self.list(
            brackets,
            args,
            |af, item, tail, _lcx| format(af, item, tail),
            ListOptions {
                omit_open_bracket: true,
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
                    horizontal: HorizontalListStrategy {
                        contents_max_width: Some(WIDTH_THRESHOLDS.fn_call_width),
                        ..
                    },
                    vertical: VerticalListStrategy {
                        wrap_to_fit: format_string_pos.map(|format_string_pos| {
                            WrapToFit {
                                format_string_pos: Some(format_string_pos),
                                // todo rename/consolidate this variable
                                max_element_width: Some(
                                    NonZero::new(
                                        WIDTH_THRESHOLDS.short_array_element_width_threshold,
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
        self.backtrack()
            .next(|_| {
                let _guard = self.single_line_guard();
                let width_limit_guard = self.width_limit_guard(WIDTH_THRESHOLDS.fn_call_width)?;
                self.expr(expr)?;
                self.out.token_space(",")?;
                self.pat(pat)?;
                if let Some(guard) = guard {
                    self.out.space_token_space("if")?;
                    self.expr(guard)?;
                }
                drop(width_limit_guard);
                self.out.token_skip_if_present(",")?;
                self.out.token_replace(")")?;
                self.tail(tail)?;
                Ok(())
            })
            .next(|_| {
                {
                    let _guard = self.indent_guard();
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    self.expr(expr)?;
                    self.out.token(",")?;
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    let (pat_first_line, pat_start) = self.out.line_col();
                    self.pat(pat)?;
                    let pat_width =
                        (self.out.line() == pat_first_line).then(|| self.out.col() - pat_start);
                    // todo width limit on pat + guard single line?
                    if let Some(guard) = guard {
                        // todo introduce constant
                        let allow_same_line = pat_width.is_some_and(|w| w <= 40);
                        self.backtrack()
                            .next_if(allow_same_line, |_| {
                                let _guard = self.could_wrap_indent_guard();
                                self.out.space_token_space("if")?;
                                self.expr(guard)?;
                                self.out.token_maybe_missing(",")?;
                                Ok(())
                            })
                            .next(|_| {
                                let _guard = self.indent_guard();
                                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                                self.out.token_space("if")?;
                                self.expr(guard)?;
                                self.out.token_maybe_missing(",")?;
                                Ok(())
                            })
                            .result()?;
                    } else {
                        self.out.token_maybe_missing(",")?;
                    }
                }
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                self.out.token_replace(")")?;
                self.tail(tail)?;
                Ok(())
            })
            .result()
    }

    fn macro_call_semi(&self, semi: MacCallSemi, brackets: Brackets) -> FormatResult {
        match (semi, brackets) {
            (MacCallSemi::Item, Brackets::Curly) => {
                self.out.token_skip_if_present(";")?;
            }
            (MacCallSemi::Yes, _) | (MacCallSemi::Item, _) => self.out.token_maybe_missing(";")?,
            (MacCallSemi::No, _) => {}
        }
        Ok(())
    }
}
