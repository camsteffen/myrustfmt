use crate::util::line_col::line_col;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::ops::ControlFlow;
use std::path::Path;

pub type FormatResult<T = ()> = Result<T, FormatError>;

pub type FormatControlFlow<C> = ControlFlow<FormatResult, C>;

pub trait FormatResultExt<T> {
    fn break_err(self) -> FormatControlFlow<T>;
}

impl<T> FormatResultExt<T> for FormatResult<T> {
    fn break_err(self) -> FormatControlFlow<T> {
        match self {
            Ok(value) => ControlFlow::Continue(value),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

macro_rules! return_if_break {
    ($control_flow:expr) => {{
        use std::ops::ControlFlow;
        match $control_flow {
            ControlFlow::Break(value) => return value,
            ControlFlow::Continue(value) => value,
        }
    }};
}
pub(crate) use return_if_break;

#[derive(Debug)]
pub enum FormatError {
    Constraint(ConstraintError),
    Parse(ParseError),
}

#[derive(Clone, Copy, Debug)]
pub enum ConstraintError {
    // todo rename to NextStrategy?
    Logical,
    NewlineNotAllowed,
    WidthLimitExceeded,
}

#[derive(Clone, Copy, Debug)]
pub struct NewlineNotAllowedError;

#[derive(Clone, Copy, Debug)]
pub struct WidthLimitExceededError;

pub type ParseResult<T = ()> = Result<T, ParseError>;

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub backtrace: Box<Backtrace>,
}

impl ParseError {
    #[cold]
    pub fn new(kind: ParseErrorKind) -> Self {
        ParseError {
            kind,
            backtrace: Box::new(Backtrace::capture()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    ExpectedPosition(usize),
    ExpectedToken(String),
    UnsupportedSyntax,
}

impl FormatError {
    pub fn display(&self, source: &str, pos: usize, path: Option<&Path>) -> impl Display {
        display_from_fn(move |f| {
            let (line, col) = line_col(source, pos);
            match path {
                None => write!(f, "Error formatting at {line}:{col}, ")?,
                Some(path) => write!(f, "Error formatting {}:{line}:{col}, ", path.display(),)?,
            }
            let next_token = |f: &mut Formatter<'_>| {
                let remaining = &source[pos..];
                let token = rustc_lexer::tokenize(remaining).next().unwrap();
                let token_str = &remaining[..token.len as usize];
                write!(f, ". Next token is `{token_str}`")
            };
            match self {
                FormatError::Constraint(ConstraintError::Logical) => {
                    write!(f, "unhandled logical constraint error")?
                }
                FormatError::Constraint(ConstraintError::WidthLimitExceeded) => {
                    write!(f, "width limit exceeded")?
                }
                FormatError::Constraint(ConstraintError::NewlineNotAllowed) => {
                    // todo
                    write!(f, "width limit exceeded")?
                }
                FormatError::Parse(parse_error) => {
                    match parse_error.kind {
                        ParseErrorKind::ExpectedPosition(expected_pos) => {
                            write!(
                                f,
                                "expected position is {} bytes {}",
                                expected_pos.abs_diff(pos),
                                if expected_pos > pos {
                                    "ahead"
                                } else {
                                    "behind"
                                }
                            )?;
                            next_token(f)?;
                        }
                        ParseErrorKind::ExpectedToken(ref token) => {
                            write!(f, "expected token: `{}`", token)?;
                            next_token(f)?;
                        }
                        ParseErrorKind::UnsupportedSyntax => {
                            write!(f, "unsupported syntax")?;
                        }
                    }
                    write!(f, "\n{}", parse_error.backtrace)?;
                }
            }
            Ok(())
        })
    }
}

impl From<ConstraintError> for FormatError {
    fn from(e: ConstraintError) -> Self {
        FormatError::Constraint(e)
    }
}

impl From<ParseError> for FormatError {
    fn from(e: ParseError) -> Self {
        FormatError::Parse(e)
    }
}

impl From<NewlineNotAllowedError> for ConstraintError {
    fn from(_: NewlineNotAllowedError) -> Self {
        ConstraintError::NewlineNotAllowed
    }
}

impl From<WidthLimitExceededError> for ConstraintError {
    fn from(_: WidthLimitExceededError) -> Self {
        ConstraintError::WidthLimitExceeded
    }
}

impl From<NewlineNotAllowedError> for FormatError {
    fn from(e: NewlineNotAllowedError) -> Self {
        FormatError::Constraint(ConstraintError::from(e))
    }
}

impl From<WidthLimitExceededError> for FormatError {
    fn from(e: WidthLimitExceededError) -> Self {
        FormatError::Constraint(ConstraintError::from(e))
    }
}

fn display_from_fn(f: impl Fn(&mut Formatter<'_>) -> std::fmt::Result) -> impl Display {
    struct Impl<F>(F);
    impl<F> Display for Impl<F>
    where
        F: Fn(&mut Formatter<'_>) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.0(f)
        }
    }
    Impl(f)
}
