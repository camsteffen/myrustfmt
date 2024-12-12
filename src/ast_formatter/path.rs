use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

use crate::ast_formatter::last_line::{EndReserved, Tail, drop_end_reserved};
use crate::ast_formatter::list::AngleBracketedArgsConfig;
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter<'_> {
    pub fn qpath(&mut self, qself: &Option<P<ast::QSelf>>, path: &ast::Path) -> FormatResult {
        self.qpath_end(qself, path, Tail::None)
    }

    pub fn qpath_end(
        &mut self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        end: Tail,
    ) -> FormatResult {
        if let Some(qself) = qself.as_deref() {
            todo!();
        }
        self.path_end(path, end)
    }

    pub fn path(&mut self, path: &ast::Path) -> FormatResult {
        self.path_end(path, Tail::None)
    }

    pub fn path_end(&mut self, path: &ast::Path, end: Tail) -> FormatResult {
        if let [first_segment, rest @ ..] = &path.segments[..] {
            self.path_segment(first_segment)?;
            for segment in rest {
                self.out.token_expect("::")?;
                self.path_segment(segment)?;
            }
        }
        self.tail(end)
    }

    pub fn path_segment(&mut self, segment: &ast::PathSegment) -> FormatResult {
        self.ident(segment.ident)?;
        if let Some(args) = &segment.args.as_deref() {
            match args {
                ast::AngleBracketed(args) => {
                    self.list(
                        &args.args,
                        |this, arg| match arg {
                            ast::AngleBracketedArg::Arg(arg) => this.generic_arg(arg),
                            ast::AngleBracketedArg::Constraint(AssocItemConstraint) => todo!(),
                        },
                        AngleBracketedArgsConfig,
                        Tail::None
                    )?;
                }
                ast::Parenthesized(parenthesized_args) => {
                    self.parenthesized_args(parenthesized_args)?;
                }
                ast::ParenthesizedElided(Span) => todo!(),
            }
        }
        Ok(())
    }

    fn generic_arg(&mut self, arg: &ast::GenericArg) -> FormatResult {
        match &arg {
            ast::GenericArg::Lifetime(lifetime) => self.lifetime(lifetime),
            ast::GenericArg::Type(ty) => self.ty(ty),
            ast::GenericArg::Const(anon_const) => todo!(),
        }
    }
}
