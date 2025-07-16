use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::tail::Tail;
use crate::num::HSize;
use crate::whitespace::VerticalWhitespaceMode;
use std::num::NonZero;

pub enum ListStrategies<'a, Item> {
    Horizontal(HorizontalListStrategy),
    Vertical(VerticalListStrategy<'a, Item>),
    Flexible(FlexibleListStrategy<'a, Item>),
}

impl<Item> ListStrategies<'_, Item> {
    pub fn horizontal() -> Self {
        ListStrategies::Horizontal(HorizontalListStrategy { .. })
    }

    pub fn horizontal_overflow() -> Self {
        ListStrategies::Horizontal(HorizontalListStrategy { overflow: true, .. })
    }

    pub fn flexible_overflow() -> Self {
        ListStrategies::Flexible(FlexibleListStrategy {
            horizontal: HorizontalListStrategy { overflow: true, .. },
            ..
        })
    }

    pub fn vertical() -> Self {
        ListStrategies::Vertical(VerticalListStrategy { .. })
    }

    pub fn get_horizontal_mut(&mut self) -> Option<&mut HorizontalListStrategy> {
        match self {
            ListStrategies::Horizontal(horizontal)
            | ListStrategies::Flexible(FlexibleListStrategy { horizontal, .. }) => Some(horizontal),
            ListStrategies::Vertical(_) => None,
        }
    }

    pub fn get_vertical(&self) -> Option<&VerticalListStrategy<'_, Item>> {
        match self {
            ListStrategies::Horizontal(_) => None,
            ListStrategies::Vertical(vertical)
            | ListStrategies::Flexible(FlexibleListStrategy { vertical, .. }) => Some(vertical),
        }
    }
}

pub struct HorizontalListStrategy {
    pub contents_max_width: Option<HSize> = None,
    pub overflow: bool = false,
}

pub struct VerticalListStrategy<'a, Item> {
    pub item_requires_own_line: Option<Box<dyn Fn(&Item) -> bool + 'a>> = None,
    pub whitespace_between: VerticalWhitespaceMode = VerticalWhitespaceMode::Break,
    pub wrap_to_fit: Option<WrapToFit> = None,
}

impl<Item> VerticalListStrategy<'_, Item> {
    pub fn wrap_to_fit(max_element_width: Option<HSize>) -> Self {
        VerticalListStrategy {
            wrap_to_fit: Some(WrapToFit {
                format_string_pos: None,
                max_element_width: max_element_width
                    .map(|v| NonZero::new(v).expect("wrap-to-fit max width must not be zero")),
            }),
            ..
        }
    }
}

pub struct FlexibleListStrategy<'a, Item> {
    pub horizontal: HorizontalListStrategy = HorizontalListStrategy { .. },
    pub vertical: VerticalListStrategy<'a, Item> = VerticalListStrategy { .. },
}

#[derive(Clone, Copy)]
pub struct WrapToFit {
    pub format_string_pos: Option<u8> = None,
    pub max_element_width: Option<NonZero<HSize>> = None,
}

pub struct ListOptions<'ast, 'tail, Item> {
    pub force_trailing_comma: bool = false,
    pub is_struct: bool = false,
    pub omit_open_bracket: bool = false,
    pub rest: Option<ListRest<'ast>> = None,
    pub strategies: ListStrategies<'ast, Item> = ListStrategies::Flexible(FlexibleListStrategy {
        ..
    }),
    pub tail: Tail<'tail, 'ast> = None,
}
