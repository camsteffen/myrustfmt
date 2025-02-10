pub struct ListItemContext {
    pub index: usize,
    pub strategy: ListStrategy,
}

pub enum ListStrategy {
    SingleLine,
    WrapToFit,
    SeparateLines,
}