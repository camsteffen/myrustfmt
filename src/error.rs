use crate::constraints::VStruct;
use crate::rustc_span::Pos;
use crate::util::display::display_from_fn;
use crate::util::line_col::line_col;
use rustc_span::BytePos;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub type FormatResult<T = ()> = Result<T, FormatError>;

#[derive(Debug)]
pub struct FormatError {
    pub kind: FormatErrorKind,
    #[cfg(debug_assertions)]
    pub backtrace: Box<Backtrace>,
    pub context_version: u32,
}

impl FormatError {
    fn backtrace(&self) -> &Backtrace {
        #[cfg(debug_assertions)]
        { &self.backtrace }
        #[cfg(not(debug_assertions))]
        {
            static DISABLED: Backtrace = Backtrace::disabled();
            &DISABLED
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormatErrorKind {
    /// Used to explicitly fail the current strategy for implementation-specific reasons
    Logical,
    // todo propagate this error when using checkpoints
    UnsupportedSyntax,
    Vertical(VerticalError),
    // todo do we actually need cause here? debug only?
    VStruct {
        vstruct: VStruct,
        cause: VerticalError,
    },
    WidthLimitExceeded,
}

/// Occurs when we attempt to write a newline while single line mode is enabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VerticalError {
    LineComment,
    MultiLineComment,
    Newline,
}

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
    write!(
        f,
        "{}: {:?}",
        error_formatting_at(source, pos, path),
        e.kind,
    )?;
    if cfg!(debug_assertions) && path.is_none() {
        write!(f, "\nSource:\n{source}")?;
    }
    write!(f, "\nformat error backtrace:\n{}", e.backtrace())?;
    Ok(())
}

#[track_caller]
pub fn panic_parse_error(error: ParseError, path: Option<&Path>, source: &str, pos: BytePos) -> ! {
    panic!(
        "{}",
        display_from_fn(move |f| write_parse_error(f, error, path, source, pos)),
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
                },
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

impl From<WidthLimitExceededError> for FormatErrorKind {
    fn from(_: WidthLimitExceededError) -> Self {
        FormatErrorKind::WidthLimitExceeded
    }
}
