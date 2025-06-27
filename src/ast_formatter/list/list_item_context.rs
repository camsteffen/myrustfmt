use crate::ast_formatter::backtrack::BacktrackCtxt;

pub struct ListItemContext<'a> {
    pub bctx: &'a BacktrackCtxt,
    pub index: usize,
    pub is_vertical: bool,
}
