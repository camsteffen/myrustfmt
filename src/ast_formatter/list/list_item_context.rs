use crate::Recover;

pub struct ListItemContext<'a> {
    pub horizontal: Option<&'a Recover>,
    pub index: usize,
}
