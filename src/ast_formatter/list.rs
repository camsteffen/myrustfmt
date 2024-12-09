use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult, SourceFormatter};

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

impl<'a> AstFormatter<'a> {
    pub fn list<T>(
        &mut self,
        kind: ListKind,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
    ) -> FormatResult {
        self.out.token_expect(kind.starting_brace())?;
        if list.is_empty() {
            self.out.token_expect(kind.ending_brace())?;
            return Ok(());
        }
        self.fallback_chain("list")
            .next("single line", |this| {
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                if kind.should_pad_contents() {
                    this.out.space()?;
                }
                format_item(this, head)?;
                for item in tail {
                    this.out.token_maybe_missing(",")?;
                    this.out.space()?;
                    format_item(this, item)?;
                }
                if kind.should_pad_contents() {
                    this.out.space()?;
                }
                this.out.token_expect(kind.ending_brace())?;
                Ok(())
            })
            .next("wrapping to fit", |this| {
                let format_item = |this: &mut Self, item: &T| {
                    this.with_width_limit_single_line(10, |this| format_item(this, item))
                };
                this.constraints().increment_indent();
                this.out.newline_indent()?;
                let [head, tail @ ..] = list else {
                    unreachable!()
                };
                format_item(this, head)?;
                this.out.token_maybe_missing(",")?;
                for item in tail {
                    this.fallback_chain("list item")
                        .next("same line", |this| {
                            this.out.space()?;
                            format_item(this, item)?;
                            this.out.token_maybe_missing(",")?;
                            Ok(())
                        })
                        .next("wrap", |this| {
                            this.out.newline_indent()?;
                            format_item(this, item)?;
                            this.out.token_maybe_missing(",")?;
                            Ok(())
                        })
                        .result()?;
                }
                this.constraints().decrement_indent();
                this.out.newline_indent()?;
                this.out.token_expect(kind.ending_brace())?;
                Ok(())
            })
            .next("separate lines", |this| {
                this.constraints().increment_indent();
                for item in list {
                    this.out.newline_indent()?;
                    format_item(this, item)?;
                    this.out.token_maybe_missing(",")?;
                }
                this.constraints().decrement_indent();
                this.out.newline_indent()?;
                this.out.token_expect(kind.ending_brace())?;
                Ok(())
            })
            .result()?;
        Ok(())
    }
}
