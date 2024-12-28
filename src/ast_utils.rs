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

pub fn is_call_or_prefixed(expr: &ast::Expr) -> bool {
    match expr.kind {
        ast::ExprKind::Call(..) | ast::ExprKind::MacCall(..) => true,
        ast::ExprKind::AddrOf(_, _, ref expr)
        | ast::ExprKind::Try(ref expr)
        | ast::ExprKind::Unary(_, ref expr)
        | ast::ExprKind::Cast(ref expr, _) => is_call_or_prefixed(expr),
        _ => false,
    }
}
