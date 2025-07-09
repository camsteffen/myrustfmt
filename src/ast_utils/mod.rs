pub mod use_tree_order;
pub mod version_sort;

use rustc_ast::ast;
use rustc_span::sym;

pub fn is_jump_expr(expr: &ast::Expr) -> bool {
    match expr.kind {
        ast::ExprKind::Become(..)
        | ast::ExprKind::Break(..)
        | ast::ExprKind::Continue(..)
        | ast::ExprKind::Ret(..)
        | ast::ExprKind::Yeet(..)
        | ast::ExprKind::Yield(..) => true,
        _ => false,
    }
}

macro_rules! postfix_utils {
    ($($kind:ident$fields:tt, $receiver:expr, $is_dot:literal,)*) => {
        macro_rules! postfix_expr_kind {
            () => ($(::rustc_ast::ast::ExprKind::$kind(..))|*);
        }
        pub(crate) use postfix_expr_kind;

        pub fn is_postfix_expr(expr: &ast::Expr) -> bool {
            matches!(expr.kind, $(::rustc_ast::ast::ExprKind::$kind(..))|*)
        }

        /// If the given expression is postfix, returns its receiver expression.
        pub fn postfix_expr_receiver(postfix_expr: &ast::Expr) -> &ast::Expr {
            match &postfix_expr.kind {
                $(::rustc_ast::ast::ExprKind::$kind$fields => $receiver,)|*
                _ => panic!("expected a postfix expression"),
            }
        }

        /// Postfix expressions with a dot can be wrapped to the next line
        pub fn postfix_expr_is_dot(postfix_expr: &ast::Expr) -> bool {
            match postfix_expr.kind {
                $(::rustc_ast::ast::ExprKind::$kind(..) => $is_dot,)|*
                _ => panic!("expected a postfix expression"),
            }
        }
    };
}
postfix_utils! {
    // note: ExprKind::Cast isn't here since it is lower precedence and so it doesn't chain
    Await(receiver, _),      receiver,              true,
    Field(receiver, _),      receiver,              true,
    MethodCall(method_call), &method_call.receiver, true,
    Index(receiver, _, _),   receiver,              false,
    Try(receiver),           receiver,              false,
}

pub fn is_rustfmt_skip(attr: &ast::Attribute) -> bool {
    attr.path_matches(&[sym::rustfmt, sym::skip])
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
