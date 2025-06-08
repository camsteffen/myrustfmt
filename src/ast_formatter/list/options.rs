use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::tail::Tail;
use crate::num::HSize;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ListShape {
    #[default]
    Flexible,
    FlexibleWithOverflow,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Default)]
pub enum ListWrapToFit {
    #[default]
    No,
    Yes { max_element_width: Option<HSize> },
}

pub struct ListOptions<'ast, 'tail, Item> {
    pub(super) rest: Option<ListRest<'ast>>,
    pub(super) shape: ListShape,
    pub(super) tail: Tail<'tail, 'ast>,
    pub(super) force_trailing_comma: bool,
    /// Called with the last item in the list. Returns true if that item always prefers overflow
    /// to being wrapped to the next line.
    pub(super) item_prefers_overflow: Option<Box<dyn Fn(&Item) -> bool>>,
    pub(super) item_requires_own_line: Option<Box<dyn Fn(&Item) -> bool>>,
    pub(super) omit_open_brace: bool,
    pub(super) single_line_max_contents_width: Option<HSize>,
    pub(super) wrap_to_fit: ListWrapToFit,
}

impl<'ast, 'tail, Item> ListOptions<'ast, 'tail, Item> {
    pub fn new() -> Self {
        ListOptions {
            rest: None,
            shape: ListShape::default(),
            tail: None,
            force_trailing_comma: false,
            item_prefers_overflow: None,
            item_requires_own_line: None,
            omit_open_brace: false,
            single_line_max_contents_width: None,
            wrap_to_fit: ListWrapToFit::default(),
        }
    }

    pub fn rest(self, rest: Option<ListRest<'ast>>) -> Self {
        ListOptions { rest, ..self }
    }

    pub fn shape(self, shape: ListShape) -> Self {
        ListOptions { shape, ..self }
    }

    pub fn tail(self, tail: Tail<'tail, 'ast>) -> ListOptions<'ast, 'tail, Item> {
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
            item_prefers_overflow: Some(Box::new(item_prefers_overflow)),
            ..self
        }
    }

    pub fn item_requires_own_line(
        self,
        item_requires_own_line: impl Fn(&Item) -> bool + 'static,
    ) -> Self {
        Self {
            item_requires_own_line: Some(Box::new(item_requires_own_line)),
            ..self
        }
    }

    pub fn omit_open_brace(self) -> Self {
        ListOptions {
            omit_open_brace: true,
            ..self
        }
    }

    pub fn single_line_max_contents_width(self, width: HSize) -> Self {
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
