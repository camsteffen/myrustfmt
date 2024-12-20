use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{AngleBracketedListConfig, list};
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter {
    pub fn qpath(&self, qself: &Option<P<ast::QSelf>>, path: &ast::Path) -> FormatResult {
        self.qpath_end(qself, path, Tail::NONE)
    }

    pub fn qpath_end(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        end: Tail<'_>,
    ) -> FormatResult {
        if let Some(_qself) = qself.as_deref() {
            todo!();
        }
        self.path_end(path, end)
    }

    pub fn path(&self, path: &ast::Path) -> FormatResult {
        self.path_end(path, Tail::NONE)
    }

    pub fn path_end(&self, path: &ast::Path, end: Tail<'_>) -> FormatResult {
        if let [first_segment, rest @ ..] = &path.segments[..] {
            self.path_segment(first_segment)?;
            for segment in rest {
                self.out.token_expect("::")?;
                self.path_segment(segment)?;
            }
        }
        self.tail(end)
    }

    pub fn path_segment(&self, segment: &ast::PathSegment) -> FormatResult {
        self.ident(segment.ident)?;
        self.generic_args(segment.args.as_deref())?;
        Ok(())
    }

    fn generic_args(&self, generic_args: Option<&ast::GenericArgs>) -> FormatResult {
        let Some(generic_args) = generic_args else {
            return Ok(());
        };
        match generic_args {
            ast::GenericArgs::AngleBracketed(args) => list(
                &args.args,
                |arg| self.angle_bracketed_arg(arg),
                AngleBracketedListConfig,
            )
            .format(self),
            // (A, B) -> C
            ast::GenericArgs::Parenthesized(parenthesized_args) => {
                self.parenthesized_args(parenthesized_args)
            }
            ast::GenericArgs::ParenthesizedElided(_span) => todo!(),
        }
    }

    fn angle_bracketed_arg(&self, arg: &ast::AngleBracketedArg) -> FormatResult {
        match arg {
            ast::AngleBracketedArg::Arg(arg) => self.generic_arg(arg),
            ast::AngleBracketedArg::Constraint(constraint) => {
                self.assoc_item_constraint(constraint)
            }
        }
    }

    fn assoc_item_constraint(&self, constraint: &ast::AssocItemConstraint) -> FormatResult {
        self.ident(constraint.ident)?;
        self.generic_args(constraint.gen_args.as_ref())?;
        match &constraint.kind {
            ast::AssocItemConstraintKind::Bound { bounds } => self.generic_bounds(bounds),
            ast::AssocItemConstraintKind::Equality { term } => {
                self.out.space()?;
                self.out.token_expect("=")?;
                self.out.space()?;
                match term {
                    ast::Term::Const(anon_const) => todo!(),
                    ast::Term::Ty(ty) => self.ty(ty),
                }
            },
        }
    }
}
