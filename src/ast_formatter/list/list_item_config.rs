use std::marker::PhantomData;

pub trait ListItemConfig: Copy {
    type Item;

    fn item_requires_own_line(_item: &Self::Item) -> bool {
        false
    }
}

pub struct DefaultListItemConfig<T>(PhantomData<T>);

impl<T> Clone for DefaultListItemConfig<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for DefaultListItemConfig<T> {}

impl<T> Default for DefaultListItemConfig<T> {
    fn default() -> Self {
        DefaultListItemConfig(PhantomData)
    }
}

impl<T> ListItemConfig for DefaultListItemConfig<T> {
    type Item = T;
}
