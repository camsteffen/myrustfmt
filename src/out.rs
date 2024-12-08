use crate::format_tree::{FormatTreeNode, ListKind};
use tracing::instrument;

pub struct Out {
    out: String,
    allow_break: bool,
    last_line_start: usize,
    max_width: Option<usize>,
    indent: usize,
}

pub fn format_tree(tree: &Vec<FormatTreeNode>, max_width: usize) -> String {
    let mut out = Out::new(max_width);
    match out.token_list(tree) {
        Ok(()) => {}
        Err(_) => {
            todo!("no good");
            out.out.clear();
            out.last_line_start = 0;
            out.max_width = None;
            match out.token_list(tree) {
                Ok(()) => {}
                Err(_) => {
                    unreachable!("too wide error with no max width")
                }
            }
        }
    }
    out.out
}

const INDENT_WIDTH: usize = 4;

pub struct OutSnapshot {
    len: usize,
    indent: usize,
    last_line_start: usize,
}

impl Out {
    pub fn new(max_width: usize) -> Out {
        Out {
            out: String::new(),
            allow_break: true,
            last_line_start: 0,
            max_width: Some(max_width),
            indent: 0,
        }
    }
    
    pub fn finish(self) -> String {
        self.out
    }
    
    fn token_list(&mut self, list: &Vec<FormatTreeNode>) -> OutResult {
        list.iter().try_for_each(|node| self.node(node))
    }

    pub fn snapshot(&self) -> OutSnapshot {
        OutSnapshot {
            len: self.out.len(),
            indent: self.indent,
            last_line_start: self.last_line_start,
        }
    }



    // #[instrument(skip(self), ret)]
    fn node(&mut self, node: &FormatTreeNode) -> OutResult {
        match node {
            FormatTreeNode::Token(token) => self.token(token),
            FormatTreeNode::List(kind, list) => self.list(kind, list),
            FormatTreeNode::Space => self.token(" "),
            FormatTreeNode::WrapIndent(left, right) => {
                self.token_list(left)?;
                self.fallback(&[
                    // one line
                    &|this| {
                        this.token(" ")?;
                        this.with_no_breaks(|this| this.token_list(right))?;
                        Ok(())
                    },
                    // wrap with one line
                    &|this| {
                        this.increment_indent();
                        this.newline_indent()?;
                        this.with_no_breaks(|this| this.token_list(right))?;
                        Ok(())
                    },
                    // allow breaks
                    &|this| {
                        this.token(" ")?;
                        this.token_list(right)?;
                        Ok(())
                    },
                    // wrap and allow breaks
                    &|this| {
                        this.increment_indent();
                        this.newline_indent()?;
                        this.token_list(right)?;
                        Ok(())
                    },
                ])?;
                Ok(())
            }
        }
    }

    fn with_no_breaks<T>(&mut self, f: impl FnOnce(&mut Out) -> T) -> T {
        let allow_break_prev = std::mem::replace(&mut self.allow_break, false);
        let result = f(self);
        self.allow_break = allow_break_prev;
        result
    }
    
    fn restore(&mut self, snapshot: &OutSnapshot) {
        self.indent = snapshot.indent;
        self.last_line_start = snapshot.last_line_start;
        self.out.truncate(snapshot.len);
    }

    // TODO static dispatch
    //  fallback_chain(|| initial).or_else(|| another_try).result()
    //  fallback_chain(|c| { c.next(|| ..); c.next(|| ..); })
    fn fallback(&mut self, funcs: &[&dyn Fn(&mut Out) -> OutResult]) -> OutResult {
        let snapshot = self.snapshot();
        if funcs.iter().any(|func| {
            match func(self) {
                Ok(()) => true,
                Err(_) => {
                    self.restore(&snapshot);
                    false
                },
            }
        }) {
            Ok(())
        } else {
            // TODO is this appropriate?
            Err(OutError::TooWide)
        }
    }

    pub fn list(&mut self, kind: &ListKind, list: &Vec<FormatTreeNode>) -> OutResult {
        self.token(kind.starting_brace())?;
        if list.is_empty() {
            // nada
        } else {
            self.fallback(&[
                // all in one line
                &|this| {
                    let [head @ .., tail] = list.as_slice() else {
                        unreachable!()
                    };
                    if kind.should_pad_contents() {
                        this.token(" ")?;
                    }
                    for item in head {
                        this.node(item)?;
                        this.token(", ")?;
                    }
                    this.node(tail)?;
                    if kind.should_pad_contents() {
                        this.token(" ")?;
                    }
                    this.token(kind.ending_brace())?;
                    Ok(())
                },
                // block indent and wrapping as needed
                &|this| {
                    this.increment_indent();
                    this.newline_indent()?;
                    let [head, tail @ ..] = list.as_slice() else {
                        unreachable!()
                    };
                    this.node(head)?;
                    this.token(",")?;
                    for item in tail {
                        this.fallback(&[
                            // continue on the same line
                            &|this| {
                                this.token(" ")?;
                                this.node(item)?;
                                this.token(",")?;
                                Ok(())
                            },
                            // wrap to the next line
                            &|this| {
                                this.newline_indent()?;
                                this.node(item)?;
                                this.token(",")?;
                                Ok(())
                            },
                        ])?;
                    }
                    this.decrement_indent();
                    this.newline_indent()?;
                    this.token(kind.ending_brace())?;
                    Ok(())
                },
                // all on separate lines
                &|this| {
                    this.increment_indent();
                    for item in list {
                        this.newline_indent()?;
                        this.node(item)?;
                        this.token(",")?;
                    }
                    this.decrement_indent();
                    this.newline_indent()?;
                    this.token(kind.ending_brace())?;
                    Ok(())
                },
            ])?;
        }
        Ok(())
    }

    pub fn token(&mut self, token: &str) -> OutResult {
        self.reserve(token.len())?;
        self.out.push_str(token);
        Ok(())
    }

    fn newline_indent(&mut self) -> OutResult {
        self.newline()?;
        self.indent()?;
        Ok(())
    }

    fn newline(&mut self) -> Result<(), NewlineNotAllowedError> {
        if !self.allow_break {
            return Err(NewlineNotAllowedError);
        }
        self.out.push('\n');
        self.last_line_start = self.out.len();
        Ok(())
    }

    fn increment_indent(&mut self) {
        self.indent += INDENT_WIDTH;
    }

    fn decrement_indent(&mut self) {
        self.indent -= INDENT_WIDTH;
    }

    fn indent(&mut self) -> Result<(), TooWideError> {
        self.reserve(self.indent)?;
        self.out.extend(std::iter::repeat_n(' ', self.indent));
        Ok(())
    }

    #[instrument(skip(self), ret, fields(out = self.out))]
    fn reserve(&mut self, len: usize) -> Result<(), TooWideError> {
        if let Some(max_width) = self.max_width {
            if max_width - self.last_line_width() < len {
                return Err(TooWideError);
            }
        }
        self.out.reserve(len);
        Ok(())
    }

    #[instrument(skip(self), ret)]
    fn last_line_width(&self) -> usize {
        self.out.len() - self.last_line_start
    }
}

pub type OutResult<T = ()> = Result<T, OutError>;

#[derive(Debug)]
struct NewlineNotAllowedError;

#[derive(Debug)]
struct TooWideError;

#[derive(Clone, Copy)]
enum OutError {
    NewlineNotAllowed,
    TooWide,
}

impl From<NewlineNotAllowedError> for OutError {
    fn from(_: NewlineNotAllowedError) -> Self {
        OutError::NewlineNotAllowed
    }
}

impl From<TooWideError> for OutError {
    fn from(_: TooWideError) -> Self {
        OutError::TooWide
    }
}
