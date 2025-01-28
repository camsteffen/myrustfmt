use crate::ast_utils::expr_kind::postfix_expr_receiver;
use rustc_ast::ast;
use rustc_span::{Symbol, sym};

pub mod expr_kind {
    use rustc_ast::ast;

    #[macro_export]
    macro_rules! block_like {
        () => {
            ast::ExprKind::Block(..)
                | ast::ExprKind::ConstBlock(_)
                | ast::ExprKind::Gen(..)
                | ast::ExprKind::TryBlock(..)
        };
    }
    pub use block_like;

    #[macro_export]
    macro_rules! control_flow {
        () => {
            ::rustc_ast::ast::ExprKind::Become(..)
                | ::rustc_ast::ast::ExprKind::Break(..)
                | ::rustc_ast::ast::ExprKind::Continue(..)
                | ::rustc_ast::ast::ExprKind::Ret(..)
                | ::rustc_ast::ast::ExprKind::Yeet(..)
                | ::rustc_ast::ast::ExprKind::Yield(..)
        };
        (Some($target:pat)) => {
            ::rustc_ast::ast::ExprKind::Become($target)
                | ::rustc_ast::ast::ExprKind::Break(_, Some($target))
                | ::rustc_ast::ast::ExprKind::Ret(Some($target))
                | ::rustc_ast::ast::ExprKind::Yeet(Some($target))
                | ::rustc_ast::ast::ExprKind::Yield(Some($target))
        };
    }
    pub use control_flow;

    #[macro_export]
    macro_rules! postfix {
        () => {
            ::rustc_ast::ast::ExprKind::Await(..)
                | ::rustc_ast::ast::ExprKind::Field(..)
                | ::rustc_ast::ast::ExprKind::Index(..)
                | ::rustc_ast::ast::ExprKind::MethodCall(..)
                | ::rustc_ast::ast::ExprKind::Try(..)
        };
    }
    pub use postfix;

    /// If the given expression is postfix, returns its receiver expression.
    /// This MUST match the same expressions as the postfix! macro defined above.
    pub fn postfix_expr_receiver(postfix_expr: &ast::Expr) -> Option<&ast::Expr> {
        match &postfix_expr.kind {
            ::rustc_ast::ast::ExprKind::Await(receiver, _)
            | ::rustc_ast::ast::ExprKind::Field(receiver, _)
            | ::rustc_ast::ast::ExprKind::Index(receiver, _, _)
            | ::rustc_ast::ast::ExprKind::Try(receiver) => Some(receiver),
            ::rustc_ast::ast::ExprKind::MethodCall(method_call) => Some(&method_call.receiver),
            _ => None,
        }
    }
}

/// Returns true if the given arm body expression requires to be wrapped in a block.
/// For false cases, we may still decide to add a block for more dynamic reasons.
pub fn arm_body_requires_block(expr: &ast::Expr) -> bool {
    match &expr.kind {
        // if/for/while headers get their own line for scan-ability
        // Also `if` could be easily mistaken for a guard otherwise
        ast::ExprKind::If(..) | ast::ExprKind::ForLoop { .. } | ast::ExprKind::While(..) => true,

        // prefix/postfix operations - see the underlying expression
        ::rustc_ast::ast::ExprKind::AddrOf(_, _, target)
        | ::rustc_ast::ast::ExprKind::Unary(_, target)
        | ::rustc_ast::ast::ExprKind::Cast(target, _)
        | expr_kind::control_flow!(Some(target)) => arm_body_requires_block(target),
        expr_kind::postfix!() => arm_body_requires_block(postfix_expr_receiver(expr).unwrap()),

        // everything else - no block required
        _ => false,
    }
}

pub fn is_rustfmt_skip(attr: &ast::Attribute) -> bool {
    static PATH: [Symbol; 2] = [sym::rustfmt, sym::skip];
    attr.path_matches(&PATH)
}

pub fn is_plain_block(expr: &ast::Expr) -> bool {
    plain_block(expr).is_some()
}

// a block with no label, no `async`, no `unsafe`
pub fn plain_block(expr: &ast::Expr) -> Option<&ast::Block> {
    match &expr.kind {
        ast::ExprKind::Block(block, None)
            if matches!(block.rules, ast::BlockCheckMode::Default) =>
        {
            Some(block)
        }
        _ => None,
    }
}
