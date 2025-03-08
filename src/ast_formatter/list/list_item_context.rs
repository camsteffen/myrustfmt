pub struct ListItemContext {
    pub len: usize,
    pub index: usize,
    pub strategy: ListStrategy,
}

impl ListItemContext {
    pub fn is_last(&self) -> bool {
        self.index == self.len - 1
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ListStrategy {
    SingleLine,
    WrapToFit,
    SeparateLines,
}
