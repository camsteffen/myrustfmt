use rustc_ast::ast;

pub fn expr_only_block(block: &ast::Block) -> Option<&ast::Expr> {
    if let [stmt] = &block.stmts[..] {
        if let ast::StmtKind::Expr(expr) = &stmt.kind {
            if expr.attrs.is_empty() {
                return Some(expr);
            }
        }
    }
    None
}
