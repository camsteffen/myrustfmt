use crate::util::line_col::line_col;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::path::Path;

#[allow(unused)]
macro_rules! debug_err {
    ($af:expr, $result:expr) => {{
        let result = $result;
        if result.is_err() {
            $af.out.with_taken_buffer(|b| {
                dbg!(b);
            });
        }
        result
    }};
}
#[allow(unused)]
pub(crate) use debug_err;

pub type FormatResult<T = ()> = Result<T, FormatError>;

pub trait FormatResultExt<T> {
    fn constraint_err_only(self) -> Result<Result<T, ConstraintError>, ParseError>;
}

impl<T> FormatResultExt<T> for FormatResult<T> {
    fn constraint_err_only(self) -> Result<Result<T, ConstraintError>, ParseError> {
        match self {
            Err(FormatError::Parse(e)) => Err(e),
            Err(FormatError::Constraint(e)) => Ok(Err(e)),
            Ok(value) => Ok(Ok(value)),
        }
    }
}

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
            write!(f, "Error formatting at ")?;
            if let Some(path) = path {
                write!(f, "{}:", path.display())?
            }
            write!(f, "{line}:{col}, ")?;
            let next_token = |f: &mut Formatter<'_>| {
                let remaining = &source[pos..];
                if let Some(token) = rustc_lexer::tokenize(remaining).next() {
                    let token_str = &remaining[..token.len as usize];
                    write!(f, ". Next token is `{token_str}`")?;
                } else {
                    write!(f, ". Reached end of file")?;
                }
                Ok(())
            };
            let backtrace = match self {
                FormatError::Constraint(e) => {
                    match e {
                        ConstraintError::Logical => {
                            write!(f, "unhandled logical constraint error")?
                        }
                        ConstraintError::NewlineNotAllowed => write!(f, "newline not allowed")?,
                        ConstraintError::WidthLimitExceeded => write!(f, "width limit exceeded")?,
                    }
                    None
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
                    Some(&parse_error.backtrace)
                }
            };
            if cfg!(debug_assertions) {
                write!(f, "\nSource:\n{}", source)?;
            }
            if let Some(backtrace) = backtrace {
                write!(f, "\n{}", backtrace)?;
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
