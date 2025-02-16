pub struct ListItemContext {
    pub len: usize,
    pub index: usize,
    pub strategy: ListStrategy,
}

#[derive(Clone, Copy, Debug)]
pub enum ListStrategy {
    SingleLine,
    WrapToFit,
    SeparateLines,
}
