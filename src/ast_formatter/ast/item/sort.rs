use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::ast::item::MaybeItem;
use crate::ast_formatter::ast::item::use_tree::order::use_tree_order;
use crate::ast_formatter::util::ast::item_lo_with_attrs;
use crate::ast_formatter::util::sort::version_sort;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_span::BytePos;
use std::cmp::Ordering;

enum NodeOrSortableItemGroup<'a, T> {
    Node(&'a T),
    SortableItemGroup(SortableItemGroupKind, &'a [T]),
}

#[derive(Clone, Copy)]
enum SortableItemGroupKind {
    Mod,
    Use,
}

impl SortableItemGroupKind {
    fn compare(self, a: &ast::Item, b: &ast::Item) -> Ordering {
        match self {
            SortableItemGroupKind::Mod => version_sort(a.ident.as_str(), b.ident.as_str()),
            SortableItemGroupKind::Use => {
                fn expect_use_tree(item: &ast::Item) -> &ast::UseTree {
                    match &item.kind {
                        ast::ItemKind::Use(use_tree) => use_tree,
                        _ => unreachable!(),
                    }
                }
                use_tree_order(expect_use_tree(a), expect_use_tree(b))
            }
        }
    }
}

impl AstFormatter {
    pub fn list_with_item_sorting<T: MaybeItem>(
        &self,
        list: &[T],
        format: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        for (i, node_or_group) in self.iter_with_sortable_item_groups(list).enumerate() {
            if i > 0 {
                self.out.newline_indent(VerticalWhitespaceMode::Between)?;
            }
            match node_or_group {
                NodeOrSortableItemGroup::Node(node) => format(node)?,
                NodeOrSortableItemGroup::SortableItemGroup(kind, group) => {
                    self.sortable_item_group(kind, group, &format)?
                }
            }
        }
        Ok(())
    }

    fn iter_with_sortable_item_groups<'a, T: MaybeItem>(
        &self,
        list: &'a [T],
    ) -> impl Iterator<Item = NodeOrSortableItemGroup<'a, T>> {
        let mut remaining = list;
        std::iter::from_fn(move || {
            let Some(item) = remaining.first()?.as_item() else {
                let first = remaining.split_off_first().unwrap();
                return Some(NodeOrSortableItemGroup::Node(first));
            };
            if is_external_mod(item) {
                let group = self.split_off_contiguous_maybe_item(&mut remaining, is_external_mod);
                Some(
                    NodeOrSortableItemGroup::SortableItemGroup(SortableItemGroupKind::Mod, group),
                )
            } else if is_use(item) {
                let group = self.split_off_contiguous_maybe_item(&mut remaining, is_use);
                Some(
                    NodeOrSortableItemGroup::SortableItemGroup(SortableItemGroupKind::Use, group),
                )
            } else {
                let first = remaining.split_off_first().unwrap();
                Some(NodeOrSortableItemGroup::Node(first))
            }
        })
    }

    fn split_off_contiguous_maybe_item<'a, T>(
        &self,
        slice: &mut &'a [T],
        filter: impl Fn(&ast::Item) -> bool,
    ) -> &'a [T]
    where
        T: MaybeItem,
    {
        self.split_off_contiguous_group(
            slice,
            |t| t.as_item().is_some_and(&filter),
            |t| item_lo_with_attrs(t.as_item().unwrap()),
            |t| t.as_item().unwrap().span.hi(),
        )
    }

    fn split_off_contiguous_group<'a, T>(
        &self,
        remaining: &mut &'a [T],
        filter: impl Fn(&T) -> bool,
        get_lo: impl Fn(&T) -> BytePos,
        get_hi: impl Fn(&T) -> BytePos,
    ) -> &'a [T] {
        let first = remaining.first().unwrap();
        let source_file = &self.out.source_reader.source_file;
        let mut line_hi = source_file
            .lookup_line(source_file.relative_position(get_hi(first)))
            .unwrap();
        let more_count = remaining[1..]
            .iter()
            .take_while(|item| {
                if !filter(item) {
                    return false;
                }
                let next_lo = source_file
                    .lookup_line(source_file.relative_position(get_lo(item)))
                    .unwrap();
                if next_lo - line_hi > 1 {
                    return false;
                }
                line_hi = source_file
                    .lookup_line(source_file.relative_position(get_hi(item)))
                    .unwrap();
                true
            })
            .count();
        remaining.split_off(..1 + more_count).unwrap()
    }

    fn sortable_item_group<T: MaybeItem>(
        &self,
        kind: SortableItemGroupKind,
        group: &[T],
        format: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        let mut sorted = Vec::from_iter(group);
        sorted.sort_by(|a, b| kind.compare(a.as_item().unwrap(), b.as_item().unwrap()));
        for (i, element) in sorted.into_iter().enumerate() {
            if i > 0 {
                self.out.newline_indent(VerticalWhitespaceMode::SingleNewline)?;
            }
            let item = element.as_item().unwrap();
            self.out.source_reader.goto(item_lo_with_attrs(item));
            format(element)?;
        }
        let last_item = group.last().unwrap().as_item().unwrap();
        self.out.source_reader.goto(last_item.span.hi());
        Ok(())
    }
}

fn is_external_mod(item: &ast::Item) -> bool {
    matches!(item.kind, ast::ItemKind::Mod(_, ast::ModKind::Unloaded))
}

fn is_use(item: &ast::Item) -> bool {
    matches!(item.kind, ast::ItemKind::Use(_))
}
