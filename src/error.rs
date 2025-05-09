use crate::rustc_span::Pos;
use crate::util::display::display_from_fn;
use crate::util::line_col::line_col;
use rustc_span::BytePos;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub type FormatResult<T = ()> = Result<T, ConstraintError>;

#[derive(Debug)]
pub struct ConstraintError {
    pub kind: ConstraintErrorKind,
    #[cfg(debug_assertions)]
    pub backtrace: Box<Backtrace>,
}

impl ConstraintError {
    pub fn new(kind: ConstraintErrorKind) -> ConstraintError {
        ConstraintError {
            kind,
            #[cfg(debug_assertions)]
            backtrace: Box::new(Backtrace::capture()),
        }
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        #[cfg(debug_assertions)]
        { Some(&self.backtrace) }
        #[cfg(not(debug_assertions))]
        { None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstraintErrorKind {
    LineCommentNotAllowed,
    MultiLineCommentNotAllowed,
    /// Returned when we know that there is a fallback strategy that is preferred
    NextStrategy,
    NewlineNotAllowed,
    WidthLimitExceeded,
}

#[derive(Clone, Copy, Debug)]
pub struct NewlineNotAllowedError;

#[derive(Clone, Copy, Debug)]
pub struct WidthLimitExceededError;

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    ExpectedPosition(BytePos),
    ExpectedToken(&'static str),
    UnexpectedEof,
}

pub fn error_formatting_at(source: &str, pos: BytePos, path: Option<&Path>) -> String {
    let path_str = path
        .map(|p| format!("{}:", p.display()))
        .unwrap_or_default();
    let (line, col) = line_col(source, pos);
    format!("Error formatting at {path_str}{line}:{col}")
}

fn write_constraint_error(
    f: &mut Formatter,
    e: &ConstraintError,
    source: &str,
    pos: BytePos,
    path: Option<&Path>,
) -> std::fmt::Result {
    write!(f, "{}, ", error_formatting_at(source, pos, path))?;
    match e.kind {
        ConstraintErrorKind::LineCommentNotAllowed => write!(f, "line comment not allowed")?,
        ConstraintErrorKind::MultiLineCommentNotAllowed => {
            write!(f, "multi-line comment not allowed")?
        }
        ConstraintErrorKind::NextStrategy => write!(f, "unhandled NextStrategy error")?,
        ConstraintErrorKind::NewlineNotAllowed => write!(f, "newline not allowed")?,
        ConstraintErrorKind::WidthLimitExceeded => write!(f, "width limit exceeded")?,
    }
    if cfg!(debug_assertions) && path.is_none() {
        write!(f, "\nSource:\n{source}")?;
    }
    if let Some(backtrace) = e.backtrace() {
        write!(f, "\nformat error backtrace:\n{backtrace}")?;
    }
    Ok(())
}

#[track_caller]
pub fn panic_parse_error(error: ParseError, path: Option<&Path>, source: &str, pos: BytePos) -> ! {
    panic!("{}",
           display_from_fn(move |f| write_parse_error(f, error, path, source, pos))
    )
}

fn write_parse_error(
    f: &mut Formatter,
    error: ParseError,
    path: Option<&Path>,
    source: &str,
    pos: BytePos,
) -> std::fmt::Result {
    write!(f, "{}, ", error_formatting_at(source, pos, path))?;
    let next_token = |f: &mut Formatter| {
        let remaining = &source[pos.to_usize()..];
        if let Some(token) = rustc_lexer::tokenize(remaining).next() {
            let token_str = &remaining[..token.len.try_into().unwrap()];
            write!(f, ". Next token is `{token_str}`")?;
        } else {
            write!(f, ". Reached end of file")?;
        }
        Ok(())
    };
    match error {
        ParseError::ExpectedPosition(expected_pos) => {
            write!(
                f,
                "expected position is {} bytes {}",
                expected_pos.to_u32().abs_diff(pos.to_u32()),
                if expected_pos > pos {
                    "ahead"
                } else {
                    "behind"
                }
            )?;
            next_token(f)?;
        }
        ParseError::ExpectedToken(ref token) => {
            write!(f, "expected token: `{token}`")?;
            next_token(f)?;
        }
        ParseError::UnexpectedEof => {
            write!(f, "unexpected EOF")?;
            next_token(f)?;
        }
    }
    if cfg!(debug_assertions) && path.is_none() {
        write!(f, "\nSource:\n{source}")?;
    }
    Ok(())
}

impl ConstraintError {
    pub fn display(&self, source: &str, pos: BytePos, path: Option<&Path>) -> impl Display {
        display_from_fn(move |f| write_constraint_error(f, self, source, pos, path))
    }
}

impl From<ConstraintErrorKind> for ConstraintError {
    fn from(kind: ConstraintErrorKind) -> Self {
        ConstraintError::new(kind)
    }
}

impl From<NewlineNotAllowedError> for ConstraintErrorKind {
    fn from(_: NewlineNotAllowedError) -> Self {
        ConstraintErrorKind::NewlineNotAllowed
    }
}

impl From<NewlineNotAllowedError> for ConstraintError {
    fn from(_: NewlineNotAllowedError) -> Self {
        ConstraintErrorKind::NewlineNotAllowed.into()
    }
}

impl From<WidthLimitExceededError> for ConstraintErrorKind {
    fn from(_: WidthLimitExceededError) -> Self {
        ConstraintErrorKind::WidthLimitExceeded
    }
}

impl From<WidthLimitExceededError> for ConstraintError {
    fn from(e: WidthLimitExceededError) -> Self {
        ConstraintErrorKind::from(e).into()
    }
}
