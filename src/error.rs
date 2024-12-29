use std::fmt::{Display, Formatter};

pub type FormatResult<T = ()> = Result<T, FormatError>;

pub trait FormatResultExt {
    fn is_ok_or_parse_error(&self) -> bool;
}

impl<T> FormatResultExt for FormatResult<T> {
    fn is_ok_or_parse_error(&self) -> bool {
        match self {
            Ok(_) => true,
            Err(e) => match e {
                FormatError::Parse(_) => true,
                FormatError::Constraint(_) => false,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum FormatError {
    Constraint(ConstraintError),
    Parse(ParseError),
}

#[derive(Clone, Copy, Debug)]
pub enum ConstraintError {
    NewlineNotAllowed,
    WidthLimitExceeded,
}

#[derive(Clone, Copy, Debug)]
pub struct NewlineNotAllowedError;

#[derive(Clone, Copy, Debug)]
pub struct WidthLimitExceededError;

pub type ParseResult<T = ()> = Result<T, ParseError>;

#[derive(Clone, Debug)]
pub enum ParseError {
    ExpectedPosition(usize),
    ExpectedToken(String),
    UnsupportedSyntax,
}

impl FormatError {
    pub fn display(&self, source: &str, pos: usize) -> impl Display {
        struct FormatErrorDisplay<'err, 'source> {
            error: &'err FormatError,
            source: &'source str,
            pos: usize,
            line: usize,
            col: usize,
        }
        impl Display for FormatErrorDisplay<'_, '_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "Error formatting at {}:{}, ", self.line, self.col)?;
                let next_token = |f: &mut Formatter<'_>| {
                    let remaining = &self.source[self.pos..];
                    let token = rustc_lexer::tokenize(remaining).next().unwrap();
                    let token_str = &remaining[..token.len as usize];
                    write!(f, ". Next token is `{token_str}`")
                };
                match *self.error {
                    FormatError::Constraint(ConstraintError::WidthLimitExceeded) => {
                        write!(f, "width limit exceeded")?
                    }
                    FormatError::Constraint(ConstraintError::NewlineNotAllowed) => {
                        write!(f, "width limit exceeded")?
                    }
                    FormatError::Parse(ParseError::ExpectedPosition(pos)) => {
                        write!(
                            f,
                            "expected position is {} bytes {}",
                            pos.abs_diff(self.pos),
                            if pos > self.pos { "ahead" } else { "behind" }
                        )?;
                        next_token(f)?;
                    }
                    FormatError::Parse(ParseError::ExpectedToken(ref token)) => {
                        write!(f, "expected token: `{}`", token)?;
                        next_token(f)?;
                    }
                    FormatError::Parse(ParseError::UnsupportedSyntax) => {
                        write!(f, "unsupported syntax")?;
                    }
                }
                Ok(())
            }
        }
        let (line, col) = line_col(source, pos);
        FormatErrorDisplay {
            error: self,
            source,
            pos,
            line,
            col,
        }
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
