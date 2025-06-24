use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::tail::Tail;
use crate::num::HSize;
use std::num::NonZero;

pub enum ListStrategies<Item> {
    Horizontal(HorizontalListStrategy),
    Vertical(VerticalListStrategy<Item>),
    Flexible(FlexibleListStrategy<Item>),
}

impl<Item> ListStrategies<Item> {
    pub fn horizontal() -> Self {
        ListStrategies::Horizontal(HorizontalListStrategy::SingleLine)
    }

    pub fn horizontal_overflow() -> Self {
        ListStrategies::Horizontal(HorizontalListStrategy::Overflow)
    }

    pub fn flexible() -> Self {
        ListStrategies::Flexible(FlexibleListStrategy { .. })
    }

    pub fn flexible_overflow() -> Self {
        ListStrategies::Flexible(FlexibleListStrategy {
            horizontal: HorizontalListStrategy::Overflow,
            ..
        })
    }

    pub fn vertical() -> Self {
        ListStrategies::Vertical(VerticalListStrategy { .. })
    }

    pub(super) fn get_vertical(&self) -> Option<&VerticalListStrategy<Item>> {
        match self {
            ListStrategies::Horizontal(_) => None,
            ListStrategies::Vertical(vertical)
            | ListStrategies::Flexible(FlexibleListStrategy { vertical, .. }) => Some(vertical),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum HorizontalListStrategy {
    #[default]
    SingleLine,
    Overflow,
}

pub struct VerticalListStrategy<Item> {
    pub item_requires_own_line: Option<Box<dyn Fn(&Item) -> bool>> = None,
    pub wrap_to_fit: Option<WrapToFit> = None,
}

impl<Item> VerticalListStrategy<Item> {
    pub fn wrap_to_fit(max_element_width: Option<HSize>) -> Self {
        VerticalListStrategy {
            wrap_to_fit: Some(WrapToFit {
                format_string_pos: None,
                max_element_width: max_element_width.map(|v| {
                    NonZero::new(v)
                        .expect("wrap-to-fit max width must not be zero")
                }),
            }),
            ..
        }
    }
}

pub struct FlexibleListStrategy<Item> {
    pub horizontal: HorizontalListStrategy = HorizontalListStrategy::SingleLine,
    pub vertical: VerticalListStrategy<Item> = VerticalListStrategy {..},
}

#[derive(Clone, Copy)]
pub struct WrapToFit {
    pub format_string_pos: Option<u8> = None,
    pub max_element_width: Option<NonZero<HSize>> = None,
}

pub struct ListOptions<'ast, 'tail, Item> {
    pub contents_max_width: Option<HSize> = None,
    pub force_trailing_comma: bool = false,
    pub is_struct: bool = false,
    /// Called with the last item in the list. Returns true if that item always prefers overflow
    /// to being wrapped to the next line.
    pub omit_open_brace: bool = false,
    pub rest: Option<ListRest<'ast>> = None,
    pub strategies: ListStrategies<Item> = ListStrategies::Flexible(FlexibleListStrategy{..}),
    pub tail: Tail<'tail, 'ast> = None,
}
