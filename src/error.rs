use crate::util::line_col::line_col;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::path::Path;
use crate::source_formatter::SourceFormatter;

pub type FormatResult<T = ()> = Result<T, FormatError>;

pub trait FormatResultExt<T> {
    fn constraint_err_only(self) -> Result<Result<T, ConstraintError>, ParseError>;

    #[allow(unused)]
    fn debug_err(self, sf: &SourceFormatter) -> Self;
}

impl<T> FormatResultExt<T> for FormatResult<T> {
    /// If self is Ok or Err with a ConstraintError, the result is returned wrapped in Ok.
    /// If self is a non-constraint-error, it is returned in an Err (the outer Result).
    /// This is often useful since constraint errors are used to trigger a fallback strategy, but
    /// other errors indicate a critical bug, and so they are usually propagated (e.g. using `?`).
    fn constraint_err_only(self) -> Result<Result<T, ConstraintError>, ParseError> {
        match self {
            Err(FormatError::Parse(e)) => Err(e),
            Err(FormatError::Constraint(e)) => Ok(Err(e)),
            Ok(value) => Ok(Ok(value)),
        }
    }

    #[track_caller]
    fn debug_err(self, sf: &SourceFormatter) -> Self {
        if self.is_err() {
            sf.debug_buffer();
        }
        self
    }
}

#[derive(Debug)]
pub enum FormatError {
    Constraint(ConstraintError),
    Parse(ParseError),
}

#[derive(Debug)]
pub struct ConstraintError {
    pub kind: ConstraintErrorKind,
    #[cfg(debug_assertions)]
    pub backtrace: Box<Backtrace>,
    #[cfg(debug_assertions)]
    pub open_checkpoint_backtrace: Option<Box<Backtrace>>,
}

impl ConstraintError {
    pub fn new(
        kind: ConstraintErrorKind,
        #[cfg(debug_assertions)]
        open_checkpoint_backtrace: Option<Box<Backtrace>>,
    ) -> ConstraintError {
        ConstraintError {
            kind,
            #[cfg(debug_assertions)]
            backtrace: Box::new(Backtrace::capture()),
            #[cfg(debug_assertions)]
            open_checkpoint_backtrace,
        }
    }

    fn backtraces(&self) -> (Option<&Backtrace>, Option<&Backtrace>) {
        #[cfg(debug_assertions)]
        {
            (
                Some(&self.backtrace),
                self.open_checkpoint_backtrace.as_deref(),
            )
        }
        #[cfg(not(debug_assertions))]
        { (None, None) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstraintErrorKind {
    /// Returned when we know that there is a fallback strategy that is preferred
    NextStrategy,
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
                    let token_str = &remaining[..token.len.try_into().unwrap()];
                    write!(f, ". Next token is `{token_str}`")?;
                } else {
                    write!(f, ". Reached end of file")?;
                }
                Ok(())
            };
            let (backtrace, open_checkpoint_backtrace) = match self {
                FormatError::Constraint(e) => {
                    match e.kind {
                        ConstraintErrorKind::NextStrategy => {
                            write!(f, "unhandled NextStrategy error")?
                        }
                        ConstraintErrorKind::NewlineNotAllowed => write!(f, "newline not allowed")?,
                        ConstraintErrorKind::WidthLimitExceeded => {
                            write!(f, "width limit exceeded")?
                        }
                    }
                    e.backtraces()
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
                            write!(f, "expected token: `{token}`")?;
                            next_token(f)?;
                        }
                        ParseErrorKind::UnsupportedSyntax => {
                            write!(f, "unsupported syntax")?;
                        }
                    }
                    (Some(&*parse_error.backtrace), None)
                }
            };
            if cfg!(debug_assertions) && path.is_none() {
                write!(f, "\nSource:\n{source}")?;
            }
            if let Some(backtrace) = backtrace {
                write!(f, "\nformat error backtrace:\n{backtrace}")?;
            }
            if let Some(backtrace) = open_checkpoint_backtrace {
                write!(f, "\nopen checkpoint backtrace:\n{backtrace}")?;
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

impl From<ConstraintErrorKind> for ConstraintError {
    fn from(kind: ConstraintErrorKind) -> Self {
        ConstraintError::new(
            kind,
            #[cfg(debug_assertions)]
            None,
        )
    }
}

impl From<ConstraintErrorKind> for FormatError {
    fn from(kind: ConstraintErrorKind) -> Self {
        ConstraintError::from(kind).into()
    }
}

impl From<ParseError> for FormatError {
    fn from(e: ParseError) -> Self {
        FormatError::Parse(e)
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

impl From<NewlineNotAllowedError> for FormatError {
    fn from(e: NewlineNotAllowedError) -> Self {
        ConstraintError::from(ConstraintErrorKind::from(e)).into()
    }
}

impl From<WidthLimitExceededError> for FormatError {
    fn from(e: WidthLimitExceededError) -> Self {
        FormatError::Constraint(ConstraintError::from(ConstraintErrorKind::from(e)))
    }
}

fn display_from_fn(fmt: impl Fn(&mut Formatter<'_>) -> std::fmt::Result) -> impl Display {
    struct Impl<F>(F);
    impl<F> Display for Impl<F>
    where
        F: Fn(&mut Formatter<'_>) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.0(f)
        }
    }
    Impl(fmt)
}
