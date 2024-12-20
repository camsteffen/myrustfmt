use rustc_span::{BytePos, Pos};
use std::fmt::{Display, Formatter};
use thiserror::Error;

pub type FormatResult<T = ()> = Result<T, FormatError>;

#[derive(Debug)]
pub struct FormatError {
    pub kind: FormatErrorKind,
    pub pos: BytePos,
}

#[derive(Clone, Debug)]
pub enum FormatErrorKind {
    Constraint(ConstraintError),
    Parse(ParseError),
}

#[derive(Clone, Copy, Debug)]
pub enum ConstraintError {
    NewlineNotAllowed,
    WidthLimitExceeded,
}

#[derive(Clone, Copy, Debug)]
// #[error("newline character is not allowed")]
pub struct NewlineNotAllowedError;

#[derive(Clone, Copy, Debug)]
// #[error("width limit exceeded")]
pub struct WidthLimitExceededError;

pub type ParseResult<T = ()> = Result<T, ParseError>;

#[derive(Clone, Debug, Error)]
pub enum ParseError {
    #[error("expected `{0}`")]
    ExpectedPosition(usize),
    #[error("expected `{0}`")]
    ExpectedToken(String),
}

impl FormatError {
    pub fn display(&self, source: &str) -> impl Display {
        struct FormatErrorDisplay<'err, 'source> {
            error: &'err FormatError,
            source: &'source str,
            line: usize,
            col: usize,
        }
        impl Display for FormatErrorDisplay<'_, '_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "Error formatting at {}:{}, ", self.line, self.col)?;
                let next_token = |f: &mut Formatter<'_>| {
                    let remaining = &self.source[self.error.pos.to_usize()..];
                    let token = rustc_lexer::tokenize(remaining).next().unwrap();
                    let token_str = &remaining[..token.len as usize];
                    write!(f, ". Next token is `{token_str}`")
                };
                match self.error.kind {
                    FormatErrorKind::Constraint(ConstraintError::WidthLimitExceeded) => {
                        write!(f, "width limit exceeded")?
                    }
                    FormatErrorKind::Constraint(ConstraintError::NewlineNotAllowed) => {
                        write!(f, "width limit exceeded")?
                    }
                    FormatErrorKind::Parse(ParseError::ExpectedPosition(pos)) => {
                        write!(
                            f,
                            "expected position is {} bytes {}",
                            pos.abs_diff(self.error.pos.to_usize()),
                            if pos > self.error.pos.to_usize() { "ahead" } else { "behind" }
                        )?;
                        next_token(f)?;
                    },
                    FormatErrorKind::Parse(ParseError::ExpectedToken(ref token)) => {
                        write!(f, "expected token: `{}`", token)?;
                        next_token(f)?;
                    }
                }
                Ok(())
            }
        }
        let (line, col) = line_col(source, self.pos.to_usize());
        FormatErrorDisplay { error: self, source, line, col }
    }
}

impl From<ConstraintError> for FormatErrorKind {
    fn from(e: ConstraintError) -> Self {
        FormatErrorKind::Constraint(e)
    }
}

impl From<ParseError> for FormatErrorKind {
    fn from(e: ParseError) -> Self {
        FormatErrorKind::Parse(e)
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

impl From<NewlineNotAllowedError> for FormatErrorKind {
    fn from(e: NewlineNotAllowedError) -> Self {
        FormatErrorKind::Constraint(ConstraintError::from(e))
    }
}

impl From<WidthLimitExceededError> for FormatErrorKind {
    fn from(e: WidthLimitExceededError) -> Self {
        FormatErrorKind::Constraint(ConstraintError::from(e))
    }
}

fn line_col(str: &str, pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for c in str[..pos].chars() {
        col += 1;
        if c == '\n' {
            line += 1;
            col = 1;
        }
    }
    (line, col)
}
