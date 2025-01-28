use rustc_ast::ast;
use rustc_span::{Symbol, sym};

macro_rules! block_like_expr_kind {
    () => {
        ast::ExprKind::Block(..)
            | ast::ExprKind::ConstBlock(_)
            | ast::ExprKind::Gen(..)
            | ast::ExprKind::TryBlock(..)
    };
}
pub(crate) use block_like_expr_kind;

macro_rules! control_flow_expr_kind {
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
pub(crate) use control_flow_expr_kind;

macro_rules! postfix_meta {
    ($mac:path) => {
        $mac! {
            // (ExprKind pattern, receiver expression, is_breakable)
            (Await(ref receiver, _), receiver, true),
            (Field(ref receiver, _), receiver, true),
            (Index(ref receiver, _, _), receiver, false),
            (MethodCall(ref method_call), &method_call.receiver, true),
            (Try(ref receiver), receiver, false),
        }
    };
}

macro_rules! postfix_utils {
    ($(($kind:ident $fields:tt, $receiver:expr, $breakable:literal),)*) => {
        macro_rules! postfix_expr_kind {
            () => ($(::rustc_ast::ast::ExprKind::$kind(..))|*);
        }
        pub(crate) use postfix_expr_kind;
        
        /// If the given expression is postfix, returns its receiver expression.
        pub fn postfix_expr_receiver(postfix_expr: &ast::Expr) -> Option<&ast::Expr> {
            match postfix_expr.kind {
                $(::rustc_ast::ast::ExprKind::$kind$fields => Some($receiver),)|*
                _ => None,
            }
        }

        pub fn postfix_expr_is_breakable(postfix_expr: &ast::Expr) -> Option<bool> {
            match postfix_expr.kind {
                $(::rustc_ast::ast::ExprKind::$kind(..) => Some($breakable),)|*
                _ => None,
            }
        }
    };
}

postfix_meta!(postfix_utils);

/// Returns true if the given arm body expression requires to be wrapped in a block.
/// For false cases, we may still decide to add a block later in the process.
pub fn arm_body_requires_block(expr: &ast::Expr) -> bool {
    match &expr.kind {
        // if/for/while headers get their own line for scan-ability
        // Also `if` could be easily mistaken for a guard otherwise
        ast::ExprKind::If(..) | ast::ExprKind::ForLoop { .. } | ast::ExprKind::While(..) => true,

        // prefix/postfix operations - see the underlying expression
        ::rustc_ast::ast::ExprKind::AddrOf(_, _, target)
        | ::rustc_ast::ast::ExprKind::Unary(_, target)
        | ::rustc_ast::ast::ExprKind::Cast(target, _)
        | control_flow_expr_kind!(Some(target)) => arm_body_requires_block(target),
        postfix_expr_kind!() => {
            arm_body_requires_block(postfix_expr_receiver(expr).unwrap())
        }

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
