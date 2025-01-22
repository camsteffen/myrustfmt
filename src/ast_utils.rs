use rustc_ast::ast;
use rustc_span::{Symbol, sym};

pub mod expr_kind {
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
    macro_rules! postfix {
        () => {
            $crate::ast_utils::expr_kind::postfix!(single_line)
                | $crate::ast_utils::expr_kind::postfix!(non_single_line)
        };
        (single_line) => {
            ::rustc_ast::ast::ExprKind::Await(..) | ::rustc_ast::ast::ExprKind::Try(_)
        };
        (non_single_line) => {
            ::rustc_ast::ast::ExprKind::Field(..)
                | ::rustc_ast::ast::ExprKind::Index(..)
                | ::rustc_ast::ast::ExprKind::MethodCall(_)
        };
    }
    pub use postfix;

    /// adds to one inner expression and may be formatted on one line
    #[macro_export]
    macro_rules! unary_like {
        ($target:pat) => {
            // around
            | ::rustc_ast::ast::ExprKind::Paren($target)
            // prefix
            | ::rustc_ast::ast::ExprKind::AddrOf(_, _, $target)
            | ::rustc_ast::ast::ExprKind::Unary(_, $target)
            // suffix
            | ::rustc_ast::ast::ExprKind::Await($target, _)
            | ::rustc_ast::ast::ExprKind::Cast($target, _)
            | ::rustc_ast::ast::ExprKind::Try($target)
            // control flow
            | ::rustc_ast::ast::ExprKind::Become($target)
            | ::rustc_ast::ast::ExprKind::Break(_, Some($target))
            | ::rustc_ast::ast::ExprKind::Ret(Some($target))
            | ::rustc_ast::ast::ExprKind::Yeet(Some($target))
            | ::rustc_ast::ast::ExprKind::Yield(Some($target))
        };
    }
    pub use unary_like;
}

pub fn arm_body_requires_block(expr: &ast::Expr) -> bool {
    match expr.kind {
        // todo touchy margins
        | expr_kind::postfix!(non_single_line) => false,
        // so as not to be easily confused with if guards
        ast::ExprKind::If(..) => true,
        // todo for loops and while loops should not require a block IFF the header fits on one line
        //   same with closures
        //   same for struct?
        //   same for match?
        ast::ExprKind::ForLoop { .. } | ast::ExprKind::While(..) => true,
        expr_kind::block_like!()
        // todo require single line?
        | ast::ExprKind::Binary(..)
        | ast::ExprKind::Lit(..)
        | ast::ExprKind::Type(..)
        | ast::ExprKind::Let(..)
        // todo require single line?
        | ast::ExprKind::Assign(..)
        | ast::ExprKind::AssignOp(..)
        | ast::ExprKind::Range(..)
        | ast::ExprKind::Call(..)
        | ast::ExprKind::Repeat(..)
        | ast::ExprKind::Underscore
        | ast::ExprKind::Path(..)
        | ast::ExprKind::InlineAsm(..)
        | ast::ExprKind::OffsetOf(..)
        | ast::ExprKind::Continue(..)
        | ast::ExprKind::IncludedBytes(..)
        | ast::ExprKind::FormatArgs(..)
        | ast::ExprKind::Err(..)
        | ast::ExprKind::Break(_, None)
        | ast::ExprKind::Ret(None)
        | ast::ExprKind::Yield(None)
        | ast::ExprKind::Yeet(None)
        | ast::ExprKind::Dummy
        | ast::ExprKind::Loop(..)
        | ast::ExprKind::Match(..)
        | ast::ExprKind::Closure(..)
        | ast::ExprKind::Array(..)
        | ast::ExprKind::MacCall(..)
        | ast::ExprKind::Struct(..)
        | ast::ExprKind::Tup(..) => false,
        expr_kind::unary_like!(ref target) => arm_body_requires_block(target),
        // _ => ArmBodyRequiresBlock::No,
    }
}

pub fn is_rustfmt_skip(attr: &ast::Attribute) -> bool {
    static PATH: [Symbol; 2] = [sym::rustfmt, sym::skip];
    attr.path_matches(&PATH)
}

pub fn plain_block(expr: &ast::Expr) -> Option<&ast::Block> {
    match &expr.kind {
        ast::ExprKind::Block(block, None) if matches!(block.rules, ast::BlockCheckMode::Default) => Some(block),
        _ => None,
    }
}
