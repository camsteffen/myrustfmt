use crate::ast_formatter::AstFormatter;
use crate::rustc_span::Pos;
use crate::util::whitespace_utils::is_whitespace;
use rustc_ast::ast;

impl AstFormatter {
    pub fn expr_only_block<'a>(&self, block: &'a ast::Block) -> Option<&'a ast::Expr> {
        let [stmt] = &block.stmts[..] else {
            return None;
        };
        let ast::StmtKind::Expr(expr) = &stmt.kind else {
            return None;
        };
        if !expr.attrs.is_empty() {
            return None;
        }
        let source = self.out.source();
        if !(is_whitespace(&source[block.span.lo().to_usize() + 1..expr.span.lo().to_usize()])
            && is_whitespace(&source[expr.span.hi().to_usize()..block.span.hi().to_usize() - 1]))
        {
            // there are comments before or after the expression
            return None;
        }
        Some(expr)
    }
}
