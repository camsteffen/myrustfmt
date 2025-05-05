mod order;

use self::order::use_tree_order;
use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::options::{ListWrapToFit, list_opt};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_lexer::TokenKind;
use rustc_span::{BytePos, Pos};

impl AstFormatter {
    /// A contiguous group of `use` declarations that can be sorted
    pub fn use_tree_group(&self, group: &[P<ast::Item>]) -> FormatResult {
        let mut sorted = Vec::from_iter(group.iter());
        sorted.sort_by(|a, b| use_tree_order(expect_use_tree(a), expect_use_tree(b)));
        for (i, item) in sorted.into_iter().enumerate() {
            // todo consider comments
            self.out.source_reader.goto(item.span.lo());
            self.item(item)?;
            if i < group.len() - 1 {
                self.out.newline_indent(VerticalWhitespaceMode::SingleNewline)?;
            }
        }
        // todo consider comments
        self.out.source_reader.goto(group.last().unwrap().span.hi());
        Ok(())
    }

    pub fn use_tree<'a>(&self, use_tree: &'a ast::UseTree, tail: &Tail) -> FormatResult {
        self.path(&use_tree.prefix, false)?;
        match use_tree.kind {
            ast::UseTreeKind::Glob => {
                self.out.token("::")?;
                self.out.token("*")?;
                self.tail(tail)?;
            }
            ast::UseTreeKind::Nested { ref items, span: _ } => {
                self.out.token("::")?;
                if let [(item, _)] = &items[..] {
                    self.out.skip_token("{")?;
                    self.use_tree(
                        item,
                        &self.tail_fn(|af| {
                            af.out.skip_token_if_present(",")?;
                            af.out.skip_token("}")?;
                            af.tail(tail)?;
                            Ok(())
                        }),
                    )?;
                } else {
                    self.out.token("{")?;
                    let mut sorted = Vec::from_iter(items.iter().enumerate().map(|(i, (ut, _))| {
                        let start = if i == 0 {
                            self.out.source_reader.pos()
                        } else {
                            self.nested_item_preceding_comma(items, i) + BytePos(1)
                        };
                        (ut, start)
                    }));
                    sorted.sort_by(|(a, _), (b, _)| use_tree_order(a, b));
                    self.list(
                        Braces::CurlyNoPad,
                        &sorted,
                        |af, &(ref use_tree, start), tail, lcx| {
                            af.out.source_reader.goto(start);
                            let tail = if lcx.index == sorted.len() - 1 {
                                &self.tail_fn(|af| {
                                    af.out.source_reader.goto(items.last().unwrap().0.span.hi());
                                    af.tail(tail)?;
                                    Ok(())
                                })
                            } else {
                                tail
                            };
                            af.use_tree(use_tree, tail)?;
                            Ok(())
                        },
                        list_opt()
                            .omit_open_brace()
                            .item_requires_own_line(|(use_tree, _): &(&ast::UseTree, _)| {
                                matches!(use_tree.kind, ast::UseTreeKind::Nested { .. })
                            })
                            .wrap_to_fit(ListWrapToFit::Yes {
                                max_element_width: None,
                            })
                            .tail(tail),
                    )?;
                }
            }
            ast::UseTreeKind::Simple(rename) => {
                if let Some(rename) = rename {
                    self.out.space_token_space("as")?;
                    self.ident(rename)?;
                }
                self.tail(tail)?;
            }
        }
        Ok(())
    }

    fn nested_item_preceding_comma(
        &self,
        items: &[(ast::UseTree, ast::NodeId)],
        index: usize,
    ) -> BytePos {
        let prev_item_end = items[index - 1].0.span.hi();
        let distance_to_comma =
            rustc_lexer::tokenize(&self.out.source_reader.source()[prev_item_end.to_usize()..])
                .map_while(|token| match token.kind {
                    TokenKind::BlockComment { .. }
                    | TokenKind::LineComment { .. }
                    | TokenKind::Whitespace => Some(token.len),
                    TokenKind::Comma => None,
                    _ => panic!("Could not find preceding comma in nested use tree"),
                })
                .sum::<u32>();
        BytePos(prev_item_end.to_u32() + distance_to_comma)
    }
}

fn expect_use_tree(item: &ast::Item) -> &ast::UseTree {
    match &item.kind {
        ast::ItemKind::Use(use_tree) => use_tree,
        _ => unreachable!(),
    }
}
