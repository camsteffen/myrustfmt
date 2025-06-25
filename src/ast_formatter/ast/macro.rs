use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, ListOptions, ListStrategies, VerticalListStrategy, WrapToFit,
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
            MacroArgs::ExprList(ref args) => {
                self.macro_args_list(args, None, tail, |af, expr, tail| af.expr_tail(expr, tail))?
            }
            MacroArgs::Format {
                ref args,
                format_string_pos,
            } => self.macro_args_list(args, Some(format_string_pos), tail, |af, item, tail| {
                af.expr_tail(item, tail)
            })?,
            MacroArgs::Matches(ref expr, ref pat) => {
                self.backtrack()
                    .next(|| {
                        self.out.with_recover_width(|| {
                            self.with_single_line(|| {
                                self.out.token("(")?;
                                self.expr(expr)?;
                                self.out.token_space(",")?;
                                self.pat(pat)?;
                                self.out.token_skip_if_present(",")?;
                                self.out.token(")")?;
                                self.tail(tail)?;
                                Ok(())
                            })
                        })
                    })
                    .next(|| {
                        self.out.token("(")?;
                        self.indented(|| {
                            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                            self.expr(expr)?;
                            self.out.token(",")?;
                            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                            self.pat(pat)?;
                            self.out.token_maybe_missing(",")?;
                            Ok(())
                        })?;
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token(")")?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .result()?;
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
                contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                strategies: ListStrategies::Flexible(FlexibleListStrategy {
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
}
