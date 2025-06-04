use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::ast_utils::{is_postfix_expr, postfix_expr_is_dot, postfix_expr_receiver};
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::HSize;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

// In rustfmt, this is called chain_width, and is 60 by default
const POSTFIX_CHAIN_MAX_WIDTH: HSize = 60;
/// Don't apply chain max width unless the chain item's distance from the start
/// of the chain is at least this much.
const POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH: HSize = 15;

/// If overflow output is this tall or more, keep it without comparing to vertical
const OVERFLOW_ONLY_HEIGHT: u32 = 7;

struct PostfixItem<'a> {
    /// The first item in the chain has the root expression, which is not a postfix expression.
    /// Subsequent items have a dot-notation item here, like `.field` or `.method()`.
    root_or_dot_item: &'a ast::Expr,
    /// any `?` or `[index]` occurring after `root_or_dot_item`
    non_dot_items: Vec<&'a ast::Expr>,
}

impl AstFormatter {
    pub fn postfix_chain(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        let chain = build_postfix_chain(expr);
        let mut chain = chain.as_slice();
        let (start_line, start_col) = self.out.line_col();

        // items that start within the first indent-width on the first line
        let indent_margin = self.out.total_indent.get() + INDENT_WIDTH;
        let multi_line_root = loop {
            let next = chain.split_off_first().unwrap();
            if chain.is_empty() {
                return self.postfix_item_tail(next, tail);
            }
            self.has_vstruct(VStruct::NonBlockIndent, || self.postfix_item(next))?;
            if self.out.line() != start_line {
                break true;
            }
            if self.out.col() > indent_margin {
                break false;
            }
        };

        if multi_line_root {
            // no indent
            self.postfix_chain_vertical(chain, tail)
        } else {
            self.backtrack()
                .next(|| {
                    self.out.with_recover_width(|| {
                        self.postfix_chain_horizontal(chain, start_col, tail)
                    })
                })
                .next(|| {
                    self.has_vstruct(VStruct::NonBlockIndent, || {
                        self.indented(|| self.postfix_chain_vertical(chain, tail))
                    })
                })
                .result()
        }
    }

    fn postfix_chain_horizontal(
        &self,
        chain: &[PostfixItem],
        start_col: HSize,
        tail: Tail,
    ) -> FormatResult {
        let last = chain.last().unwrap();
        if !matches!(last.root_or_dot_item.kind, ast::ExprKind::MethodCall(_)) {
            // this chain is not overflowable, so simply format it on one line
            self.with_single_line(|| self.postfix_items(chain, start_col))?;
            self.tail(tail)?;
            return Ok(());
        }
        let (overflowable, before_overflow) = chain.split_last().unwrap();
        self.with_single_line(|| self.postfix_items(before_overflow, start_col))?;
        self.postfix_chain_overflow(overflowable, start_col, tail)?;
        Ok(())
    }

    fn postfix_chain_overflow(
        &self,
        overflowable: &PostfixItem,
        start_col: HSize,
        tail: Tail,
    ) -> FormatResult {
        // self.with_chain_width_limit(start_col, || {
        //     self.simulate_wrap_indent(true, || {
        //         self.postfix_item(overflowable)
        //     })
        // })
        let first_line = self.out.line();
        let checkpoint = self.out.checkpoint();
        self.with_chain_width_limit(start_col, || self.postfix_item(overflowable))?;
        let overflow_height = self.out.line() - first_line + 1;
        self.tail(tail)?;
        if overflow_height == 1 || overflow_height >= OVERFLOW_ONLY_HEIGHT {
            return Ok(());
        }
        let overflow_lookahead = self.out.capture_lookahead(&checkpoint);

        // try writing the overflowable on the next line to measure its height in separate lines format
        // todo share logic with match arm
        // todo should we check if the wrap allows a *longer* first line, like match arm?
        // todo account for having less width if the fallback will add a block
        
        /*
        Only do horizontal if zero method call args
        Prefer vertical chain if wrap indent causes longer first line
          (either by allowing horizontal method call arguments
            OR by allowing longer first line within method call argument overflow)
            
        Perhaps the most questionable case is the following, where the vertical chain is preferred,
        even though has more lines than the horizontal chain with overflow. This generally involves
        some list (in this case a list of struct fields) that may or may not be formatted in one
        line. On the plus side, there is some value in splitting the outer thing (the chain) into
        multiple lines instead of the inner thing (the struct).
        
        // Good:
        person
            .city
            .country
            .planet
            .galaxy(Foo { bar, baz });
        
        // Bad:
        person.city.country.planet.galaxy(Foo {
            bar,
            baz,
        });
        
       
         */
        let wrap_result = self.indented(|| {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            let wrap_first_line = self.out.line();
            self.postfix_item(overflowable)?;
            let height = self.out.line() - wrap_first_line + 1;
            self.tail(tail)?;
            Ok(height)
        });

        match wrap_result {
            Err(_) => {
                self.out.restore_checkpoint(&checkpoint);
                self.out.restore_lookahead(overflow_lookahead);
                Ok(())
            }
            Ok(vertical_height) => {
                if vertical_height < overflow_height && false {
                    // prefer vertical formatting
                    Err(FormatErrorKind::Logical.into())
                } else {
                    self.out.restore_checkpoint(&checkpoint);
                    self.out.restore_lookahead(overflow_lookahead);
                    Ok(())
                }
            }
        }
    }

    fn postfix_chain_vertical(&self, chain: &[PostfixItem], tail: Tail) -> FormatResult {
        for item in chain {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.postfix_item(item)?;
        }
        self.tail(tail)?;
        Ok(())
    }

    fn with_chain_width_limit(
        &self,
        start_col: HSize,
        format: impl Fn() -> FormatResult,
    ) -> FormatResult {
        let offset = self.out.col() - start_col;
        let limit = (offset >= POSTFIX_CHAIN_MIN_ITEM_OFFSET_FOR_MAX_WIDTH)
            .then_some(POSTFIX_CHAIN_MAX_WIDTH);
        self.with_width_limit_from_start_opt(start_col, limit, format)
    }

    fn postfix_item(&self, item: &PostfixItem) -> FormatResult {
        self.postfix_item_tail(item, None)
    }

    fn postfix_item_tail(&self, item: &PostfixItem, tail: Tail) -> FormatResult {
        let non_dot_items = |af: &Self| af.postfix_non_dot_items(&item.non_dot_items, tail);
        match item.root_or_dot_item.kind {
            ast::ExprKind::Await(..) => {
                self.out.token(".")?;
                self.out.token("await")?;
                non_dot_items(self)?;
            }
            ast::ExprKind::Field(_, ident) => {
                self.out.token(".")?;
                self.ident(ident)?;
                non_dot_items(self)?;
            }
            ast::ExprKind::MethodCall(ref method_call) => {
                self.out.token(".")?;
                self.path_segment(&method_call.seg, true, self.tail_token("(").as_ref())?;
                // todo this is consistent with rustfmt, but would it be better to force args to be
                //   on the same line, just allowing overflow of the last arg?
                self.call_args_after_open_paren(
                    &method_call.args,
                    self.tail_fn(non_dot_items).as_ref(),
                )?;
            }
            // root expression
            _ => {
                self.expr_tail(item.root_or_dot_item, self.tail_fn(non_dot_items).as_ref())?;
            }
        }
        Ok(())
    }

    fn postfix_items(&self, items: &[PostfixItem], start_col: HSize) -> FormatResult {
        items
            .iter()
            .try_for_each(|item| self.with_chain_width_limit(start_col, || self.postfix_item(item)))
    }

    fn postfix_non_dot_items(&self, postfix_tail: &[&ast::Expr], tail: Tail) -> FormatResult {
        if postfix_tail.is_empty() {
            return self.tail(tail);
        }
        for (i, expr) in postfix_tail.iter().enumerate() {
            let is_last = i == postfix_tail.len() - 1;
            let tail = if is_last { tail } else { None };
            match expr.kind {
                ast::ExprKind::Index(_, ref index, _) => {
                    self.out.token("[")?;
                    self.backtrack()
                        .next(|| {
                            self.with_single_line(|| {
                                self.expr(index)?;
                                self.out.token("]")?;
                                self.tail(tail)?;
                                Ok(())
                            })
                        })
                        .next(|| {
                            self.enclosed_after_opening("]", || self.expr(index))?;
                            self.tail(tail)?;
                            Ok(())
                        })
                        .result()?;
                }
                ast::ExprKind::Try(..) => {
                    self.out.token("?")?;
                    self.tail(tail)?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

fn build_postfix_chain(expr: &ast::Expr) -> Vec<PostfixItem> {
    let mut current = expr;
    let mut items = Vec::new();
    let mut non_dot_items = Vec::new();
    let root = loop {
        if postfix_expr_is_dot(current) {
            items.push(PostfixItem {
                root_or_dot_item: current,
                non_dot_items: non_dot_items.drain(..).rev().collect(),
            })
        } else {
            non_dot_items.push(current);
        }
        let receiver = postfix_expr_receiver(current);
        if !is_postfix_expr(receiver) {
            break receiver;
        }
        current = receiver;
    };
    non_dot_items.reverse();
    items.push(PostfixItem {
        root_or_dot_item: root,
        non_dot_items,
    });
    items.reverse();
    items
}
