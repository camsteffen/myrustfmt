use crate::formatter::{FormatResult, Formatter};

#[derive(Clone, Copy, Debug)]
pub enum ListKind {
    CurlyBraces,
    SquareBraces,
    Parethesis,
}

impl ListKind {
    pub fn starting_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "{",
            ListKind::Parethesis => "(",
            ListKind::SquareBraces => "[",
        }
    }

    pub fn ending_brace(self) -> &'static str {
        match self {
            ListKind::CurlyBraces => "}",
            ListKind::Parethesis => ")",
            ListKind::SquareBraces => "]",
        }
    }

    pub fn should_pad_contents(self) -> bool {
        match self {
            ListKind::CurlyBraces => true,
            ListKind::SquareBraces => false,
            ListKind::Parethesis => false,
        }
    }
}

impl<'a> Formatter<'a> {
    pub fn list<T>(
        &mut self,
        kind: ListKind,
        list: &[T],
        format_item: impl Fn(&mut Formatter<'a>, &T) -> FormatResult,
    ) -> FormatResult {
        self.token_unchecked(kind.starting_brace())?;
        if list.is_empty() {
            self.token_unchecked(kind.ending_brace())?;
            return Ok(());
        }
        self.fallback_chain("list")
            .next("single line", |this| {
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                this.optional_space(kind.should_pad_contents())?;
                format_item(this, head)?;
                for item in tail {
                    this.token_unchecked(",")?;
                    this.space()?;
                    format_item(this, item)?;
                }
                this.optional_space(kind.should_pad_contents())?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .next("wrapping to fit", |this| {
                let format_item = |this: &mut Formatter<'a>, item: &T| {
                    this.with_width_limit(10, |this| format_item(this, item))
                };
                this.out.increment_indent();
                this.newline_indent()?;
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                format_item(this, head)?;
                this.token_unchecked(",")?;
                for item in tail {
                    this.fallback_chain("list item")
                        .next("same line", |this| {
                            this.space()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        })
                        .next("wrap", |this| {
                            this.newline_indent()?;
                            format_item(this, item)?;
                            this.token_unchecked(",")?;
                            Ok(())
                        })
                        .result()?;
                }
                this.out.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .next("separate lines", |this| {
                this.out.increment_indent();
                for item in list {
                    this.newline_indent()?;
                    format_item(this, item)?;
                    this.token_unchecked(",")?;
                }
                this.out.decrement_indent();
                this.newline_indent()?;
                this.token_unchecked(kind.ending_brace())?;
                Ok(())
            })
            .result()?;
        Ok(())
    }
}
