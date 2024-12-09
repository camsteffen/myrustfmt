use tracing::instrument;

pub enum Constraint {
    SingleLine,
    SingleLineLimitWidth { pos: usize },
}

pub struct Out {
    out: String,
    // allow_break: bool,
    last_line_start: usize,
    max_width: Option<usize>,
    indent: usize,
    constraints: Vec<Constraint>,
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
            // allow_break: true,
            last_line_start: 0,
            max_width: Some(max_width),
            indent: 0,
            constraints: Vec::new(),
        }
    }

    pub fn finish(self) -> String {
        self.out
    }

    pub fn len(&self) -> usize {
        self.out.len()
    }
    
    pub fn current_indent(&self) -> usize {
        self.indent
    }
    
    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub fn snapshot(&self) -> OutSnapshot {
        OutSnapshot {
            len: self.out.len(),
            indent: self.indent,
            last_line_start: self.last_line_start,
        }
    }

    pub fn restore(&mut self, snapshot: &OutSnapshot) {
        self.indent = snapshot.indent;
        self.last_line_start = snapshot.last_line_start;
        self.out.truncate(snapshot.len);
    }

    pub fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn pop_constraint(&mut self) {
        self.constraints.pop();
    }

    /*
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
     */

    // TODO static dispatch
    //  fallback_chain(|| initial).or_else(|| another_try).result()
    //  fallback_chain(|c| { c.next(|| ..); c.next(|| ..); })
    fn fallback(&mut self, funcs: &[&dyn Fn(&mut Out) -> OutResult]) -> OutResult {
        let snapshot = self.snapshot();
        if funcs.iter().any(|func| match func(self) {
            Ok(()) => true,
            Err(_) => {
                self.restore(&snapshot);
                false
            }
        }) {
            Ok(())
        } else {
            // TODO is this appropriate?
            Err(OutError::TooWide)
        }
    }

    /**
     * A "token" must 1) fit in one line and 2) not contain comments
     */
    pub fn token(&mut self, token: &str) -> OutResult {
        for constraint in &self.constraints {
            match constraint {
                Constraint::SingleLine => {}
                Constraint::SingleLineLimitWidth { pos } => {
                    if token.len() > pos - self.out.len() {
                        return Err(OutError::TooWide);
                    }
                }
            }
        }
        self.reserve(token.len())?;
        self.out.push_str(token);
        Ok(())
    }

    pub fn newline_indent(&mut self) -> OutResult {
        self.newline()?;
        self.indent()?;
        Ok(())
    }

    pub fn newline(&mut self) -> Result<(), NewlineNotAllowedError> {
        for constraint in &self.constraints {
            match constraint {
                Constraint::SingleLine | Constraint::SingleLineLimitWidth { .. } => {
                    return Err(NewlineNotAllowedError);
                }
            }
        }
        self.out.push('\n');
        self.last_line_start = self.out.len();
        Ok(())
    }

    pub fn increment_indent(&mut self) {
        self.indent += INDENT_WIDTH;
    }

    pub fn decrement_indent(&mut self) {
        self.indent -= INDENT_WIDTH;
    }

    pub fn indent(&mut self) -> Result<(), TooWideError> {
        self.reserve(self.indent)?;
        self.out.extend(std::iter::repeat_n(' ', self.indent));
        Ok(())
    }

    #[instrument(skip(self), ret, fields(out = self.out))]
    fn reserve(&mut self, len: usize) -> Result<(), TooWideError> {
        if let Some(max_width) = self.max_width {
            if len > max_width - self.last_line_width() {
                return Err(TooWideError);
            }
        }
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

#[derive(Clone, Copy, Debug)]
pub enum OutError {
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
