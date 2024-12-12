use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

#[derive(Clone, Copy)]
pub struct EndWidth(usize);

impl EndWidth {
    pub const ZERO: EndWidth = EndWidth(0);
}

pub struct EndReserved {
    _private: (),
}

impl<'a> AstFormatter<'a> {
    pub fn with_end_width(
        &mut self,
        len: usize,
        f: impl FnOnce(&mut Self, EndWidth) -> FormatResult<EndReserved>,
    ) -> FormatResult {
        let EndReserved { .. } = f(self, EndWidth(len))?;
        Ok(())
    }

    pub fn reserve_end(&mut self, end_width: EndWidth) -> FormatResult<EndReserved> {
        let EndWidth(len) = end_width;
        self.out.require_width(len)?;
        Ok(EndReserved { _private: () })
    }
}

pub fn drop_end_reserved(_last_line: EndReserved) {}
