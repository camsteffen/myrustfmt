use crate::ast_formatter::list::ListRest;
use crate::ast_formatter::tail::Tail;
use crate::num::HSize;
use crate::util::default::default;
use std::num::NonZero;

pub enum ListStrategies {
    Horizontal(HorizontalListStrategy),
    Vertical(VerticalListStrategy),
    Flexible(HorizontalListStrategy, VerticalListStrategy),
}

impl ListStrategies {
    pub fn horizontal() -> ListStrategies {
        ListStrategies::Horizontal(HorizontalListStrategy::SingleLine)
    }

    pub fn horizontal_overflow() -> ListStrategies {
        ListStrategies::Horizontal(HorizontalListStrategy::Overflow)
    }

    pub fn flexible() -> ListStrategies {
        ListStrategies::Flexible(
            HorizontalListStrategy::SingleLine,
            VerticalListStrategy { .. },
        )
    }

    pub fn flexible_overflow() -> ListStrategies {
        ListStrategies::Flexible(
            HorizontalListStrategy::Overflow,
            VerticalListStrategy { .. },
        )
    }

    pub fn vertical() -> ListStrategies {
        ListStrategies::Vertical(default())
    }
}

impl ListStrategies {
    pub fn get_vertical(&self) -> Option<&VerticalListStrategy> {
        match self {
            ListStrategies::Horizontal(_) => None,
            ListStrategies::Vertical(vertical) | ListStrategies::Flexible(_, vertical) => {
                Some(vertical)
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum HorizontalListStrategy {
    #[default]
    SingleLine,
    Overflow,
}

impl HorizontalListStrategy {
    pub fn is_overflow(self) -> bool {
        matches!(self, Self::Overflow)
    }
}

#[derive(Default)]
pub struct VerticalListStrategy {
    pub wrap_to_fit: Option<WrapToFit> = None,
}

impl VerticalListStrategy {
    pub fn wrap_to_fit(max_element_width: Option<HSize>) -> VerticalListStrategy {
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

#[derive(Clone, Copy)]
pub struct WrapToFit {
    pub format_string_pos: Option<u8>,
    pub max_element_width: Option<NonZero<HSize>>,
}

pub struct ListOptions<'ast, 'tail, Item> {
    pub contents_max_width: Option<HSize> = None,
    pub force_trailing_comma: bool = false,
    pub is_struct: bool = false,
    /// Called with the last item in the list. Returns true if that item always prefers overflow
    /// to being wrapped to the next line.
    // todo move to flexible strategy object
    pub item_requires_own_line: Option<Box<dyn Fn(&Item) -> bool>> = None,
    pub omit_open_brace: bool = false,
    pub rest: Option<ListRest<'ast>> = None,
    pub strategies: ListStrategies = ListStrategies::Flexible(HorizontalListStrategy::SingleLine, VerticalListStrategy{..}),
    pub tail: Tail<'tail, 'ast> = None,
}
