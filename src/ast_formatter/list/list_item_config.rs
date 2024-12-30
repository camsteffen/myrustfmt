use std::marker::PhantomData;

pub trait ListItemConfig: Copy {
    type Item;

    const ITEMS_POSSIBLY_MUST_HAVE_OWN_LINE: bool = false;

    fn item_must_have_own_line(_item: &Self::Item) -> bool {
        panic!("ITEMS_POSSIBLY_MUST_HAVE_OWN_LINE is false");
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
