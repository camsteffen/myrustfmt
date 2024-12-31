use rustc_ast::ast;
use rustc_span::Symbol;
use rustc_span::sym;

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

pub fn is_rustfmt_skip(attr: &ast::Attribute) -> bool {
    static PATH: [Symbol; 2] = [sym::rustfmt, sym::skip];
    attr.path_matches(&PATH)
}

pub fn is_plain_block(expr: &ast::Expr) -> bool {
    match &expr.kind {
        ast::ExprKind::Block(block, None) => matches!(block.rules, ast::BlockCheckMode::Default),
        _ => false,
    }
}
