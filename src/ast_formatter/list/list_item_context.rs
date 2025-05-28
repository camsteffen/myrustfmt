pub struct ListItemContext {
    pub index: usize,
    pub strategy: ListStrategy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListStrategy {
    Horizontal,
    WrapToFit,
    Vertical,
}
