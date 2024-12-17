use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{list_overflow_yes, param_list_config};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::source_formatter::FormatResult;

impl AstFormatter<'_> {
    pub fn dot_chain(&mut self, expr: &ast::Expr, tail: Tail, is_overflow: bool) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let [root, dot_chain @ ..] = &dot_chain[..] else {
            unreachable!();
        };
        if is_overflow {
            // single-line the whole thing
            self.with_single_line(|this| {
                this.expr(root, Tail::NONE)?;
                dot_chain
                    .iter()
                    .try_for_each(|item| this.dot_chain_item(item))?;
                Ok(())
            })?;
            self.tail(&tail)?;
            return Ok(());
        }
        let root_is_single_line = self.with_is_single_line(|this| this.expr(root, Tail::NONE))?;
        if root_is_single_line {
            self.dot_chain_single_line_root(dot_chain, tail)
        } else {
            // each item on a separate line, no indent
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            self.tail(&tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line_root(&mut self, dot_chain: &[&ast::Expr], tail: Tail) -> FormatResult {
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|this| {
                    this.with_single_line(|this| {
                        for item in dot_chain {
                            this.dot_chain_item(item)?;
                        }
                        Ok(())
                    })
                });
                // single line until the last item
                // chain.next(|this| {
                //     let last = dot_chain.last().unwrap();
                    // let should_single_line_it = match &last.kind {
                    //     ast::ExprKind::Field(..) => true,
                    //     ast::ExprKind::MethodCall(method_call) => method_call.args.len() < 3,
                    //     _ => unreachable!(),
                    // };
                    // if should_single_line_it {
                    //     this.with_single_line(|this| {
                    //         for item in dot_chain {
                    //             this.dot_chain_item(item)?;
                    //         }
                    //         Ok(())
                    //     })?;
                    // } else {
                    //     this.with_single_line(|this| {
                    //         for item in &dot_chain[..dot_chain.len() - 1] {
                    //             this.dot_chain_item(item)?;
                    //         }
                    //         Ok(())
                    //     })?;
                    // this.with_is_in_overflow(true, |this| {
                    //     this.dot_chain_item(last)
                    // })?;
                    // this.with_height_limit(dot_chain.len() + 1, |this| {
                    //     this.dot_chain_item(last)
                    // })?;
                    // }
                    // Ok(())
                // });
                // wrap and indent each item
                chain.next(|this| {
                    this.indented(|this| {
                        for item in dot_chain {
                            this.out.newline_indent()?;
                            this.dot_chain_item(item)?;
                        }
                        Ok(())
                    })
                })
            },
            move |this| this.tail(&tail),
        )?;
        Ok(())
    }

    fn dot_chain_item(&mut self, expr: &ast::Expr) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg)?;
                let single_line_max_contents_width = RUSTFMT_CONFIG_DEFAULTS.fn_call_width;
                self.list(
                    &method_call.args,
                    |this, arg| {
                        this.expr(arg, Tail::NONE)
                    },
                    param_list_config(Some(single_line_max_contents_width)),
                    list_overflow_yes(),
                    Tail::NONE,
                )?;
                Ok(())
            }
            _ => unreachable!("invalid dot chain item ExprKind"),
        }
    }
}

fn build_dot_chain<'a>(chain: &mut Vec<&'a ast::Expr>, expr: &'a ast::Expr) {
    let inner = match &expr.kind {
        ast::ExprKind::Field(target, _ident) => Some(target),
        ast::ExprKind::MethodCall(method_call) => Some(&method_call.receiver),
        _ => None,
    };
    if let Some(inner) = inner {
        build_dot_chain(chain, inner);
    }
    chain.push(expr);
}
