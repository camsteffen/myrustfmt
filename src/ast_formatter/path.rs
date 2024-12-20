use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{AngleBracketedListConfig, list};
use rustc_ast::ast;
use rustc_ast::ptr::P;

impl AstFormatter {
    pub fn qpath(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        is_expr: bool,
    ) -> FormatResult {
        self.qpath_tail(qself, path, is_expr, Tail::NONE)
    }

    pub fn qpath_tail(
        &self,
        qself: &Option<P<ast::QSelf>>,
        path: &ast::Path,
        is_expr: bool,
        tail: Tail<'_>,
    ) -> FormatResult {
        if let Some(qself) = qself.as_deref() {
            self.out.token_expect("<")?;
            self.ty(&qself.ty)?;
            let rest = if qself.position > 0 {
                self.out.space()?;
                self.out.token_expect("as")?;
                self.out.space()?;
                let (as_path, rest) = path.segments.split_at(qself.position);
                self.path_segments(as_path, false)?;
                rest
            } else {
                &path.segments
            };
            self.out.token_expect(">")?;
            self.out.token_expect("::")?;
            self.path_segments(rest, is_expr)?;
        } else {
            self.path(path, is_expr)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    pub fn path(&self, path: &ast::Path, is_expr: bool) -> FormatResult {
        self.path_segments(&path.segments, is_expr)
    }

    pub fn path_segments(&self, segments: &[ast::PathSegment], is_expr: bool) -> FormatResult {
        let (first, rest) = segments.split_first().unwrap();
        self.path_segment(first, is_expr)?;
        for segment in rest {
            self.out.token_expect("::")?;
            self.path_segment(segment, is_expr)?;
        }
        Ok(())
    }

    pub fn path_segment(&self, segment: &ast::PathSegment, is_expr: bool) -> FormatResult {
        self.ident(segment.ident)?;
        if let Some(generic_args) = segment.args.as_deref() {
            if is_expr {
                self.out.token_expect("::")?;
            }
            self.generic_args(generic_args)?;
        };
        Ok(())
    }

    fn generic_args(&self, generic_args: &ast::GenericArgs) -> FormatResult {
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
        if let Some(generic_args) = &constraint.gen_args {
            self.generic_args(generic_args)?;
        }
        match &constraint.kind {
            ast::AssocItemConstraintKind::Bound { bounds } => self.generic_bounds(bounds),
            ast::AssocItemConstraintKind::Equality { term } => {
                self.out.space()?;
                self.out.token_expect("=")?;
                self.out.space()?;
                match term {
                    ast::Term::Const(_anon_const) => todo!(),
                    ast::Term::Ty(ty) => self.ty(ty),
                }
            }
        }
    }
}
