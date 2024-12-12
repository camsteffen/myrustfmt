use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{ArrayListConfig, ParamListConfig};
use crate::source_formatter::FormatResult;

use rustc_ast::ast;
use rustc_ast::ptr::P;

impl<'a> AstFormatter<'a> {
    pub fn expr(&mut self, expr: &ast::Expr) -> FormatResult {
        match expr.kind {
            ast::ExprKind::Array(ref items) => {
                self.list(items, |this, e| this.expr(e), ArrayListConfig)?
            }
            ast::ExprKind::ConstBlock(_) => todo!(),
            ast::ExprKind::Call(ref func, ref args) => self.call(func, args)?,
            ast::ExprKind::MethodCall(ref method_call) => self.method_call(method_call)?,
            ast::ExprKind::Tup(_) => todo!(),
            ast::ExprKind::Binary(_, _, _) => todo!(),
            ast::ExprKind::Unary(_, _) => todo!(),
            ast::ExprKind::Lit(_) => self.out.copy_span(expr.span),
            ast::ExprKind::Cast(_, _) => todo!(),
            ast::ExprKind::Type(_, _) => todo!(),
            ast::ExprKind::Let(_, _, _, _) => todo!(),
            ast::ExprKind::If(_, _, _) => todo!(),
            ast::ExprKind::While(_, _, _) => todo!(),
            ast::ExprKind::ForLoop { .. } => todo!(),
            ast::ExprKind::Loop(_, _, _) => todo!(),
            ast::ExprKind::Match(ref scrutinee, ref arms, ast::MatchKind::Prefix) => {
                self.match_(scrutinee, arms, expr)?
            }
            ast::ExprKind::Match(_, _, ast::MatchKind::Postfix) => todo!(),
            ast::ExprKind::Closure(ref closure) => self.closure(closure)?,
            ast::ExprKind::Block(ref block, label) => {
                if let Some(label) = label {
                    self.ident(label.ident)?;
                    self.out.space()?;
                }
                self.block(block)?;
            }
            ast::ExprKind::Gen(_, _, _, _) => todo!(),
            ast::ExprKind::Await(_, _) => todo!(),
            ast::ExprKind::TryBlock(_) => todo!(),
            ast::ExprKind::Assign(_, _, _) => todo!(),
            ast::ExprKind::AssignOp(_, _, _) => todo!(),
            ast::ExprKind::Field(ref expr, ident) => {
                self.expr(expr)?;
                self.out.token_expect(".")?;
                self.ident(ident)?;
            }
            ast::ExprKind::Index(_, _, _) => todo!(),
            ast::ExprKind::Range(_, _, _) => todo!(),
            ast::ExprKind::Underscore => todo!(),
            ast::ExprKind::Path(ref qself, ref path) => self.qpath(qself, path)?,
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target) => {
                self.addr_of(borrow_kind, mutability, target, expr)?
            }
            ast::ExprKind::Break(_, _) => todo!(),
            ast::ExprKind::Continue(_) => todo!(),
            ast::ExprKind::Ret(_) => todo!(),
            ast::ExprKind::InlineAsm(_) => todo!(),
            ast::ExprKind::OffsetOf(_, _) => todo!(),
            ast::ExprKind::MacCall(ref mac_call) => self.mac_call(mac_call)?,
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
        Ok(())
    }

    fn addr_of(
        &mut self,
        borrow_kind: ast::BorrowKind,
        mutability: ast::Mutability,
        target: &ast::Expr,
        expr: &ast::Expr,
    ) -> FormatResult {
        match borrow_kind {
            ast::BorrowKind::Raw => todo!(),
            ast::BorrowKind::Ref => self.out.token_at("&", expr.span.lo())?,
        }
        self.mutability(mutability)?;
        self.expr(target)?;
        Ok(())
    }

    fn call(&mut self, func: &ast::Expr, args: &[P<ast::Expr>]) -> FormatResult {
        self.expr(func)?;
        self.list(args, |this, arg| this.expr(arg), ParamListConfig)?;
        Ok(())
    }
    
    fn delim_args(&mut self, delim_args: &ast::DelimArgs) {
        self.out.copy_span(delim_args.dspan.entire());
    }
    
    pub fn mac_call(&mut self, mac_call: &ast::MacCall) -> FormatResult {
        self.path(&mac_call.path)?;
        self.out.token_expect("!")?;
        self.delim_args(&mac_call.args);
        Ok(())
    }

    fn match_(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
        expr: &ast::Expr,
    ) -> FormatResult {
        self.out.token_at("match", expr.span.lo())?;
        self.out.space()?;
        self.expr(scrutinee)?;
        self.out.space()?;
        self.out.token_expect("{")?;
        self.with_indent(|this| {
            for arm in arms {
                this.out.newline_indent()?;
                this.arm(arm)?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.out.token_expect("}")?;
        Ok(())
    }

    fn arm(&mut self, arm: &ast::Arm) -> FormatResult {
        self.attrs(&arm.attrs)?;
        self.pat(&arm.pat)?;
        if let Some(guard) = arm.guard.as_deref() {
            self.out.space()?;
            self.out.token_expect("if")?;
            self.out.space()?;
            self.expr(guard)?;
        }
        if let Some(body) = arm.body.as_deref() {
            self.out.space()?;
            self.out.token_expect("=>")?;
            self.out.space()?;
            self.expr(body)?;
            if self.out.char_ending_at(body.span.hi()) == b'}' {
                self.out.skip_token_if_present(",");
            } else {
                self.out.token_expect(",")?;
            }
        } else {
            todo!();
        }
        Ok(())
    }

    fn method_call(&mut self, method_call: &ast::MethodCall) -> FormatResult {
        self.expr(&method_call.receiver)?;
        self.out.token_expect(".")?;
        self.path_segment(&method_call.seg)?;
        self.list(
            &method_call.args,
            |this, arg| this.expr(arg),
            ParamListConfig,
        )?;
        Ok(())
    }
}
