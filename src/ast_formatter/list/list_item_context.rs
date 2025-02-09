pub struct ListItemContext {
    pub index: usize,
    pub len: usize,
    pub strategy: ListStrategy,
}

impl ListItemContext {
    pub fn is_last(&self) -> bool {
        self.index == self.len - 1
    }
}

pub enum ListStrategy {
    SingleLine,
    WrapToFit,
    SeparateLines,
}