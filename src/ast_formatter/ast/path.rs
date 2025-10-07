use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::brackets::Brackets;
use crate::ast_formatter::list::ListItemContext;
use crate::ast_formatter::list::options::ListOptions;
use crate::ast_formatter::tail::Tail;
use crate::error::{FormatErrorKind, FormatResult};
use rustc_ast::ast;

impl AstFormatter {
    pub fn qpath(
        &self,
        qself: &Option<Box<ast::QSelf>>,
        path: &ast::Path,
        turbofish: bool,
        tail: Tail,
    ) -> FormatResult {
        let Some(qself) = qself.as_deref() else {
            return self.path_tail(path, turbofish, tail);
        };
        self.out.token("<")?;
        self.ty(&qself.ty)?;
        let rest = if qself.position > 0 {
            self.out.space_token_space("as")?;
            let (as_path, rest) = path.segments.split_at(qself.position);
            self.path_segments(as_path, false)?;
            rest
        } else {
            &path.segments
        };
        self.out.token(">")?;
        self.out.token("::")?;
        self.path_segments_tail(rest, turbofish, tail)?;
        Ok(())
    }

    pub fn path(&self, path: &ast::Path, turbofish: bool) -> FormatResult {
        self.path_segments(&path.segments, turbofish)
    }

    pub fn path_tail(&self, path: &ast::Path, turbofish: bool, tail: Tail) -> FormatResult {
        self.path_segments_tail(&path.segments, turbofish, tail)
    }

    pub fn path_segments(&self, segments: &[ast::PathSegment], turbofish: bool) -> FormatResult {
        self.path_segments_tail(segments, turbofish, None)
    }

    pub fn path_segments_tail(
        &self,
        segments: &[ast::PathSegment],
        turbofish: bool,
        tail: Tail,
    ) -> FormatResult {
        match segments {
            [] => panic!("empty path segments"),
            [segment] => self.path_segment(segment, turbofish, tail)?,
            [head @ .., last] => {
                for segment in head {
                    self.path_segment(segment, turbofish, None)?;
                    self.out.token("::")?;
                }
                self.path_segment(last, turbofish, tail)?;
            }
        }
        Ok(())
    }

    /// `turbofish` is needed for expressions and patterns, but not for types and modules
    pub fn path_segment(
        &self,
        segment: &ast::PathSegment,
        turbofish: bool,
        tail: Tail,
    ) -> FormatResult {
        self.ident(segment.ident)?;
        if let Some(generic_args) = segment.args.as_deref() {
            self.generic_args(generic_args, turbofish, tail)?;
        } else {
            self.tail(tail)?;
        }
        Ok(())
    }

    fn generic_args(
        &self,
        generic_args: &ast::GenericArgs,
        turbofish: bool,
        tail: Tail,
    ) -> FormatResult {
        match generic_args {
            ast::GenericArgs::AngleBracketed(args) if args.args.is_empty() => {
                if turbofish {
                    self.out.token_skip("::")?;
                }
                self.out.token_skip("<")?;
                self.out.token_skip(">")?;
                self.tail(tail)?;
            }
            ast::GenericArgs::AngleBracketed(args) => {
                if turbofish {
                    self.out.token("::")?;
                }
                self.list(
                    Brackets::Angle,
                    &args.args,
                    Self::angle_bracketed_arg,
                    ListOptions { tail, .. },
                )?;
            }
            // (A, B) -> C
            ast::GenericArgs::Parenthesized(parenthesized_args) => {
                assert_eq!(turbofish, false);
                self.parenthesized_args(parenthesized_args, tail)?
            }
            // feature(return_type_notation)
            ast::GenericArgs::ParenthesizedElided(_span) => {
                return Err(self.err(FormatErrorKind::UnsupportedSyntax));
            }
        }
        Ok(())
    }

    fn angle_bracketed_arg(
        &self,
        arg: &ast::AngleBracketedArg,
        tail: Tail,
        _lcx: ListItemContext,
    ) -> FormatResult {
        match arg {
            ast::AngleBracketedArg::Arg(arg) => self.generic_arg(arg, tail),
            ast::AngleBracketedArg::Constraint(constraint) => {
                self.assoc_item_constraint(constraint, tail)
            }
        }
    }

    fn assoc_item_constraint(
        &self,
        constraint: &ast::AssocItemConstraint,
        tail: Tail,
    ) -> FormatResult {
        self.ident(constraint.ident)?;
        if let Some(generic_args) = &constraint.gen_args {
            self.generic_args(generic_args, false, None)?;
        }
        match &constraint.kind {
            ast::AssocItemConstraintKind::Bound { bounds } => self.generic_bounds(bounds, tail),
            ast::AssocItemConstraintKind::Equality { term } => {
                self.out.space_token_space("=")?;
                match term {
                    ast::Term::Const(anon_const) => self.expr_tail(&anon_const.value, tail),
                    ast::Term::Ty(ty) => self.ty_tail(ty, tail),
                }
            }
        }
    }
}
