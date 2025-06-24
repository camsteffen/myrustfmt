use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::options::{
    HorizontalListStrategy, ListOptions, ListStrategies, VerticalListStrategy, WrapToFit,
};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::macro_args::{MacroArgs, mac_call_id};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use std::num::NonZero;

impl AstFormatter {
    pub fn macro_call(&self, mac_call: &ast::MacCall) -> FormatResult {
        self.path(&mac_call.path, true)?;
        self.out.token("!")?;
        let args = self.module.macro_args.get(&mac_call_id(mac_call));
        if let Some(args) = args {
            self.macro_args(args)?;
        } else {
            if matches!(mac_call.args.delim, rustc_ast::token::Delimiter::Brace) {
                self.out.space()?;
            }
            self.out.copy_span(mac_call.args.dspan.entire())?;
        }
        Ok(())
    }

    fn macro_args(&self, args: &MacroArgs) -> FormatResult {
        match *args {
            MacroArgs::ExprList(ref args) => {
                self.macro_args_list(args, None, |af, expr, tail| af.expr_tail(expr, tail))?
            }
            MacroArgs::Format {
                ref args,
                format_string_pos,
            } => self.macro_args_list(args, Some(format_string_pos), |af, item, tail| {
                af.expr_tail(item, tail)
            })?,
            MacroArgs::MetaItemInner(ref args) => {
                self.macro_args_list(args, None, |af, item, tail| {
                    af.meta_item_inner(item)?;
                    af.tail(tail)?;
                    Ok(())
                })?
            }
        }
        Ok(())
    }

    fn macro_args_list<T>(
        &self,
        args: &[T],
        format_string_pos: Option<u8>,
        format: impl Fn(&Self, &T, Tail) -> FormatResult,
    ) -> FormatResult {
        self.list(
            Braces::Parens,
            &args,
            |af, item, tail, _lcx| format(af, item, tail),
            ListOptions {
                contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width),
                strategies: ListStrategies::Flexible(
                    HorizontalListStrategy::SingleLine,
                    VerticalListStrategy {
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
                    },
                ),
                ..
            },
        )
    }
}
