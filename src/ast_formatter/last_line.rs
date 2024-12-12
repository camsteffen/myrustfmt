use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

#[derive(Clone, Copy)]
pub enum Tail {
    None,
    Semicolon,
    SpaceSemicolon,
}

pub struct EndReserved {
    _private: (),
}

impl<'a> AstFormatter<'a> {
    pub fn tail(&mut self, tail: Tail) -> FormatResult {
        match tail {
            Tail::None => Ok(()),
            Tail::Semicolon => self.out.token_expect(";"),
            Tail::SpaceSemicolon => {
                self.out.space()?;
                self.out.token_expect(";")
            }
        }
    }
}

pub fn drop_end_reserved(_last_line: EndReserved) {}
