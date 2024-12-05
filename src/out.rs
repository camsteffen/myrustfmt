use crate::format_tree::{FormatTreeNode, ListKind};
use tracing::instrument;

struct Out {
    out: String,
    // is_breaking: bool,
    // is_breaking_next: bool,
    pending_fallback: bool,
    allow_break: bool,
    last_line_start: usize,
    max_width: Option<usize>,
    indent: usize,
}

pub fn format_tree(tree: &Vec<FormatTreeNode>, max_width: usize) -> String {
    let mut out = Out {
        out: String::new(),
        allow_break: true,
        pending_fallback: false,
        // is_breaking: false,
        // is_breaking_next: false,
        last_line_start: 0,
        max_width: Some(max_width),
        indent: 0,
    };
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

macro_rules! return_if_ok {
    ($e:expr) => {{
        match $e {
            Ok(val) => return Ok(val),
            Err(e) => e,
        }
    }};
}

const INDENT_WIDTH: usize = 4;

impl Out {
    fn token_list(&mut self, list: &Vec<FormatTreeNode>) -> OutResult {
        list.iter().try_for_each(|node| self.node(node))
    }

    fn checkpoint(&mut self, f: impl FnOnce(&mut Out) -> OutResult) -> OutResult {
        let len_prev = self.out.len();
        let Out {
            indent: indent_prev,
            last_line_start: last_line_start_prev,
            ..
        } = *self;
        let result = f(self);
        if let Err(_) = result {
            self.indent = indent_prev;
            self.last_line_start = last_line_start_prev;
            self.out.truncate(len_prev);
        }
        result
    }

    // fn token_list_with_retries(
    //     &mut self,
    //     list: &Vec<FormatTreeNode>,
    //     allow_breaking: bool,
    // ) -> Result<(), TooWideError> {
    //     let has_break_sooner = list
    //         .iter()
    //         .any(|node| matches!(node, FormatTreeNode::BreakSooner(_)));
    //     let with_is_breaking_next = |this: &mut Out, is_breaking_next| {
    //         this.checkpoint(|this| {
    //             this.with_is_breaking_next(is_breaking_next, |this| this.token_list(list))
    //         })
    //     };
    //     let with_is_breaking = |this: &mut Out, is_breaking| {
    //         this.with_is_breaking(is_breaking, |this| {
    //             let TooWideError = return_if_ok!(with_is_breaking_next(this, false));
    //             if has_break_sooner {
    //                 let TooWideError = return_if_ok!(with_is_breaking_next(this, true));
    //             }
    //             Err(TooWideError)
    //         })
    //     };
    //     let TooWideError = return_if_ok!(with_is_breaking(self, false));
    //     if allow_breaking && list.iter().any(|node| node.can_break()) {
    //         let TooWideError = return_if_ok!(with_is_breaking(self, true));
    //     }
    //     Err(TooWideError)
    // }

    // #[instrument(skip(self, f))]
    // fn with_is_breaking<T>(&mut self, is_breaking: bool, f: impl FnOnce(&mut Out) -> T) -> T {
    //     let is_breaking_prev = std::mem::replace(&mut self.is_breaking, is_breaking);
    //     let result = f(self);
    //     self.is_breaking = is_breaking_prev;
    //     result
    // }

    // #[instrument(skip(self, f))]
    // fn with_is_breaking_next<T>(
    //     &mut self,
    //     is_breaking_next: bool,
    //     f: impl FnOnce(&mut Out) -> T,
    // ) -> T {
    //     let is_breaking_next_prev = std::mem::replace(&mut self.is_breaking_next, is_breaking_next);
    //     let result = f(self);
    //     self.is_breaking_next = is_breaking_next_prev;
    //     result
    // }

    // #[instrument(skip(self), ret)]
    fn node(&mut self, node: &FormatTreeNode) -> OutResult {
        match node {
            FormatTreeNode::Token(token) => self.token(token),
            FormatTreeNode::List(kind, list) => self.list(kind, list),
            // FormatTreeNode::MaybeBlock(list) => {
            //     if self.is_breaking {
            //         self.increment_indent();
            //         self.newline_indent()?;
            //         self.token_list(list)?;
            //         self.decrement_indent();
            //         self.newline_indent()?;
            //     } else {
            //         self.token_list(list)?;
            //     }
            //     Ok(())
            // }
            FormatTreeNode::Space => self.token(" "),
            // FormatTreeNode::SpaceOrWrapIndent => {
            //     if self.is_breaking {
            //         self.increment_indent();
            //         self.newline_indent()?;
            //     } else {
            //         self.token(" ")?;
            //     }
            //     Ok(())
            // }
            // FormatTreeNode::BreakSooner(list) => {
            //     self.token_list_with_retries(list, self.is_breaking_next)
            // }
            // FIXME these are the same
            // FormatTreeNode::BreakLater(list) => {
            //     self.token_list_with_retries(list, self.is_breaking_next)
            // }
            FormatTreeNode::WrapIndent(left, right) => {
                self.token_list(left)?;
                self.fallback(
                    false,
                    &[
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
                    ],
                )?;
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

    fn fallback(
        &mut self,
        high_precedence: bool,
        funcs: &[&dyn Fn(&mut Out) -> OutResult],
    ) -> OutResult {
        if self.pending_fallback && !high_precedence {
            self.checkpoint(funcs[0])
        } else {
            let [head @ .., tail] = funcs else {
                panic!("funcs is empty")
            };
            self.pending_fallback = true;
            let success = head.iter().any(|func| self.checkpoint(func).is_ok());
            self.pending_fallback = false;
            if success {
                return Ok(());
            }
            self.checkpoint(tail)
        }
    }

    fn list(&mut self, kind: &ListKind, list: &Vec<FormatTreeNode>) -> OutResult {
        self.token(kind.starting_brace())?;
        if list.is_empty() {
            // nada
        } else {
            self.fallback(
                false,
                &[
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
                            this.fallback(
                                true,
                                &[
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
                                ],
                            )?;
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
                ],
            )?;
        }
        /*
        if self.is_breaking {
            if !list.is_empty() {
                self.increment_indent();
                for item in list {
                    self.newline_indent()?;
                    self.node(item)?;
                    self.token(",")?;
                }
                self.decrement_indent();
                self.newline_indent()?;
            }
        } else if let [head @ .., tail] = list.as_slice() {
            if kind.should_pad_contents() {
                self.token(" ")?;
            }
            for item in head {
                self.node(item)?;
                self.token(", ")?;
            }
            self.node(tail)?;
            if kind.should_pad_contents() {
                self.token(" ")?;
            }
        }
        self.token(kind.ending_brace())?;
         */
        Ok(())
    }

    fn token(&mut self, token: &str) -> OutResult {
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

type OutResult<T = ()> = Result<T, OutError>;

#[derive(Debug)]
struct NewlineNotAllowedError;

#[derive(Debug)]
struct TooWideError;

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
