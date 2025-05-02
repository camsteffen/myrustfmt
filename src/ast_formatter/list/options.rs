use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::tail::Tail;
use crate::num::HPos;

#[derive(Clone, Copy, PartialEq)]
pub enum ListShape {
    Flexible,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub enum ListWrapToFit {
    No,
    Yes { max_element_width: Option<HPos> },
}

pub fn list_opt<'ast, 'tail, Item>() -> ListOptions<'ast, 'tail, Item> {
    ListOptions {
        shape: ListShape::Flexible,
        rest: ListRest::None,
        tail: Tail::none(),
        item_prefers_overflow: Box::new(|_| false),
        item_requires_own_line: Box::new(|_| false),
        force_trailing_comma: false,
        omit_open_brace: false,
        single_line_max_contents_width: None,
        wrap_to_fit: ListWrapToFit::No,
    }
}

pub struct ListOptions<'ast, 'tail, Item> {
    pub rest: ListRest<'ast>,
    pub shape: ListShape,
    pub tail: &'tail Tail<'ast>,
    /// Called with the last item in the list. Returns true if that item always prefers overflow
    /// to being wrapped to the next line.
    pub item_prefers_overflow: Box<dyn Fn(&Item) -> bool>,
    pub item_requires_own_line: Box<dyn Fn(&Item) -> bool>,
    pub force_trailing_comma: bool,
    pub omit_open_brace: bool,
    pub single_line_max_contents_width: Option<HPos>,
    pub wrap_to_fit: ListWrapToFit,
}

impl<'ast, 'tail, Item> ListOptions<'ast, 'tail, Item> {
    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListOptions { rest, ..self }
    }

    pub fn shape(self, shape: ListShape) -> Self {
        ListOptions { shape, ..self }
    }

    pub fn tail(self, tail: &'tail Tail<'ast>) -> ListOptions<'ast, 'tail, Item> {
        Self { tail, ..self }
    }

    pub fn force_trailing_comma(self, force_trailing_comma: bool) -> Self {
        Self {
            force_trailing_comma,
            ..self
        }
    }

    pub fn item_prefers_overflow(
        self,
        item_prefers_overflow: impl Fn(&Item) -> bool + 'static,
    ) -> Self {
        Self {
            item_prefers_overflow: Box::new(item_prefers_overflow),
            ..self
        }
    }

    pub fn item_requires_own_line(
        self,
        item_requires_own_line: impl Fn(&Item) -> bool + 'static,
    ) -> Self {
        Self {
            item_requires_own_line: Box::new(item_requires_own_line),
            ..self
        }
    }

    pub fn omit_open_brace(self) -> Self {
        ListOptions {
            omit_open_brace: true,
            ..self
        }
    }

    pub fn single_line_max_contents_width(self, width: HPos) -> Self {
        ListOptions {
            single_line_max_contents_width: Some(width),
            ..self
        }
    }

    pub fn wrap_to_fit(self, wrap_to_fit: ListWrapToFit) -> Self {
        Self {
            wrap_to_fit,
            ..self
        }
    }
}
