use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ArrayListConfig;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn expr(&mut self, expr: &ast::Expr) -> FormatResult {
        match expr.kind {
            ast::ExprKind::Array(ref items) => {
                self.list(items, |this, e| this.expr(e), ArrayListConfig)
            }
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(_, _) => todo!(),
            ast::ExprKind::MethodCall(_) => todo!(),
            ast::ExprKind::Tup(_) => todo!(),
            ast::ExprKind::Binary(_, _, _) => todo!(),
            ast::ExprKind::Unary(_, _) => todo!(),
            ast::ExprKind::Lit(_) => todo!(),
            ast::ExprKind::Cast(_, _) => todo!(),
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(_, _, _, _) => todo!(),
            ast::ExprKind::If(_, _, _) => todo!(),
            ast::ExprKind::While(_, _, _) => todo!(),
            ast::ExprKind::ForLoop { .. } => todo!(),
            ast::ExprKind::Loop(_, _, _) => todo!(),
            ast::ExprKind::Match(_, _, _) => todo!(),
            ast::ExprKind::Closure(_) => todo!(),
            ast::ExprKind::Block(_, _) => todo!(),
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::Await(_, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(_, _, _) => todo!(),
            ast::ExprKind::AssignOp(_, _, _) => todo!(),
            ast::ExprKind::Field(_, _) => todo!(),
            ast::ExprKind::Index(_, _, _) => todo!(),
            ast::ExprKind::Range(_, _, _) => todo!(),
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.qpath(qself, path),
            ast::ExprKind::AddrOf(_, _, _) => todo!(),
            ast::ExprKind::Break(_, _) => todo!(),
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(_) => todo!(),
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(_) => todo!(),
            ast::ExprKind::Struct(_) => todo!(),
            ast::ExprKind::Repeat(_, _) => todo!(),
            ast::ExprKind::Paren(_) => todo!(),
            ast::ExprKind::Try(_) => todo!(),
            ast::ExprKind::Yield(_) => todo!(),
            ast::ExprKind::Yeet(_) => todo!(),
            ast::ExprKind::Become(_) => todo!(),
            ast::ExprKind::IncludedBytes(_) => todo!(),
            ast::ExprKind::FormatArgs(_) => todo!(),
            ast::ExprKind::Err(_) => todo!(),
            ast::ExprKind::Dummy => todo!(),
        }
    }
}
