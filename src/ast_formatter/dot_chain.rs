use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::param_list_config;
use crate::source_formatter::FormatResult;

use rustc_ast::ast;

impl AstFormatter<'_> {
    pub fn dot_chain(&mut self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        let [root, dot_chain @ ..] = &dot_chain[..] else {
            unreachable!();
        };
        let is_root_single_line = self.with_is_single_line(|this| this.expr(root, Tail::None))?;
        if dot_chain.is_empty() {
            self.tail(tail)
        } else if is_root_single_line {
            self.dot_chain_single_line_root(dot_chain, tail)
        } else {
            // each item on a separate line, no indent
            for item in dot_chain {
                self.out.newline_indent()?;
                self.dot_chain_item(item)?;
            }
            self.tail(tail)?;
            Ok(())
        }
    }

    fn dot_chain_single_line_root(&mut self, dot_chain: &[&ast::Expr], tail: Tail) -> FormatResult {
        self.fallback_chain(
            |chain| {
                // single line until the last item
                chain.next(|this| {
                    this.with_single_line(|this| {
                        for item in &dot_chain[..dot_chain.len() - 1] {
                            this.dot_chain_item(item)?;
                        }
                        Ok(())
                    })?;
                    this.dot_chain_item(dot_chain.last().unwrap())?;
                    Ok(())
                });
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
            |this| this.tail(tail),
        )?;
        Ok(())
    }

    fn dot_chain_item(&mut self, expr: &ast::Expr) -> FormatResult {
        self.out.token_expect(".")?;
        match expr.kind {
            ast::ExprKind::Field(_, ident) => self.ident(ident),
            ast::ExprKind::MethodCall(ref method_call) => {
                self.path_segment(&method_call.seg)?;
                self.list(
                    &method_call.args,
                    |this, arg| this.expr(arg, Tail::None),
                    param_list_config(),
                    Tail::None,
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
