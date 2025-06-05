use crate::rustc_span::Pos;
use crate::util::display::display_from_fn;
use crate::util::line_col::line_col;
use rustc_span::BytePos;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub type FormatResult<T = ()> = Result<T, FormatError>;

pub trait FormatResultExt {
    #[allow(unused)]
    fn debug_err(self) -> Self;
}

impl<T> FormatResultExt for FormatResult<T> {
    fn debug_err(self) -> Self {
        if let Err(e) = &self {
            eprintln!("Error: {:?}\nBacktrace:\n{}", e.kind, &e.backtrace);
        }
        self
    }
}

#[derive(Debug)]
pub struct FormatError {
    pub kind: FormatErrorKind,
    #[cfg(debug_assertions)]
    pub backtrace: Box<Backtrace>,
}

impl FormatError {
    pub fn new(kind: FormatErrorKind) -> FormatError {
        FormatError {
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

#[derive(Debug, PartialEq)]
pub enum FormatErrorKind {
    LineCommentNotAllowed,
    /// List tried to overflow when single-line constraint is enabled
    ListOverflow { cause: Box<FormatErrorKind> },
    /// Used to explicitly fail the current strategy for implementation-specific reasons
    Logical,
    MultiLineCommentNotAllowed,
    NewlineNotAllowed,
    WidthLimitExceeded,
    UnsupportedSyntax,
    VStruct { cause: Box<FormatErrorKind> },
}

impl FormatErrorKind {
    pub fn root_cause(&self) -> &Self {
        match self {
            FormatErrorKind::ListOverflow { cause } | FormatErrorKind::VStruct { cause } => {
                cause.root_cause()
            }
            FormatErrorKind::LineCommentNotAllowed
            | FormatErrorKind::MultiLineCommentNotAllowed
            | FormatErrorKind::NewlineNotAllowed
            | FormatErrorKind::Logical
            | FormatErrorKind::UnsupportedSyntax
            | FormatErrorKind::WidthLimitExceeded => self,
        }
    }

    pub fn is_vertical(&self) -> bool {
        match self {
            FormatErrorKind::LineCommentNotAllowed
            | FormatErrorKind::ListOverflow { .. }
            | FormatErrorKind::MultiLineCommentNotAllowed
            | FormatErrorKind::NewlineNotAllowed
            | FormatErrorKind::VStruct { .. } => true,

            FormatErrorKind::Logical
            | FormatErrorKind::WidthLimitExceeded
            | FormatErrorKind::UnsupportedSyntax => false,
        }
    }
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
    e: &FormatError,
    source: &str,
    pos: BytePos,
    path: Option<&Path>,
) -> std::fmt::Result {
    write!(f, "{}, ", error_formatting_at(source, pos, path))?;
    match e.kind.root_cause() {
        FormatErrorKind::LineCommentNotAllowed => write!(f, "line comment not allowed")?,
        FormatErrorKind::MultiLineCommentNotAllowed => write!(f, "multi-line comment not allowed")?,
        kind @ FormatErrorKind::Logical => write!(f, "unhandled {kind:?}")?,
        FormatErrorKind::NewlineNotAllowed => write!(f, "newline not allowed")?,
        FormatErrorKind::WidthLimitExceeded => write!(f, "width limit exceeded")?,
        FormatErrorKind::UnsupportedSyntax => write!(f, "unsupported syntax")?,
        FormatErrorKind::ListOverflow { cause: _ } | FormatErrorKind::VStruct { cause: _ } => {
            unreachable!()
        }
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

impl FormatError {
    pub fn display(&self, source: &str, pos: BytePos, path: Option<&Path>) -> impl Display {
        display_from_fn(move |f| write_constraint_error(f, self, source, pos, path))
    }
}

impl From<FormatErrorKind> for FormatError {
    fn from(kind: FormatErrorKind) -> Self {
        FormatError::new(kind)
    }
}

impl From<NewlineNotAllowedError> for FormatErrorKind {
    fn from(_: NewlineNotAllowedError) -> Self {
        FormatErrorKind::NewlineNotAllowed
    }
}

impl From<NewlineNotAllowedError> for FormatError {
    fn from(_: NewlineNotAllowedError) -> Self {
        FormatErrorKind::NewlineNotAllowed.into()
    }
}

impl From<WidthLimitExceededError> for FormatErrorKind {
    fn from(_: WidthLimitExceededError) -> Self {
        FormatErrorKind::WidthLimitExceeded
    }
}

impl From<WidthLimitExceededError> for FormatError {
    fn from(e: WidthLimitExceededError) -> Self {
        FormatErrorKind::from(e).into()
    }
}
