use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::ParamListConfig;
use crate::source_formatter::FormatResult;

use rustc_ast::ast;

impl AstFormatter<'_> {
    pub fn dot_chain(&mut self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let mut dot_chain = Vec::new();
        build_dot_chain(&mut dot_chain, expr);
        self.do_dot_chain(&dot_chain, tail)
    }

    fn do_dot_chain(&mut self, dot_chain: &[&ast::Expr], tail: Tail) -> FormatResult {
        let [root, rest @ ..] = &dot_chain else {
            unreachable!("empty dot chain")
        };
        self.fallback_chain(
            |chain| {
                chain.next(|this| this.dot_chain_with_single_line_root(root, rest));
                // root expression spans multiple lines, each item on a separate line, no indent
                chain.next(|this| {
                    this.expr(root, Tail::None)?;
                    for item in rest {
                        this.out.newline_indent()?;
                        this.dot_chain_item(item)?;
                    }
                    Ok(())
                });
            },
            |this| this.tail(tail),
        )
    }

    fn dot_chain_with_single_line_root(
        &mut self,
        root: &ast::Expr,
        rest: &[&ast::Expr],
    ) -> FormatResult {
        self.with_single_line(|this| this.expr(root, Tail::None))?;
        self.fallback_chain(
            |chain| {
                // whole chain on one line
                chain.next(|this| {
                    this.with_single_line(|this| {
                        for item in rest {
                            this.dot_chain_item(item)?;
                        }
                        Ok(())
                    })
                });
                // hanging indent and wrap each item
                chain.next(|this| {
                    this.indented(|this| {
                        for item in rest {
                            this.out.newline_indent()?;
                            this.dot_chain_item(item)?;
                        }
                        Ok(())
                    })
                })
            },
            |_| Ok(()),
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
                    ParamListConfig,
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
