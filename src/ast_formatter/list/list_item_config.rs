use std::marker::PhantomData;

pub trait ListItemConfig {
    type Item;

    fn item_requires_own_line(_item: &Self::Item) -> bool {
        false
    }

    /// Called with the last item in the list. Returns true if that item always prefers overflow
    /// to being wrapped to the next line.
    fn last_item_prefers_overflow(_item: &Self::Item) -> bool {
        false
    }
}

pub struct DefaultListItemConfig<T>(PhantomData<T>);

impl<T> Default for DefaultListItemConfig<T> {
    fn default() -> Self {
        DefaultListItemConfig(PhantomData)
    }
}

impl<T> ListItemConfig for DefaultListItemConfig<T> {
    type Item = T;
}
