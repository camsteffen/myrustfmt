use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

pub trait ListConfig {
    const START_BRACE: &'static str;
    const END_BRACE: &'static str;
    const PAD_CONTENTS: bool;

    fn max_single_line_contents_width(&self) -> Option<usize> {
        None
    }
}

pub struct ArrayListConfig;
pub struct ParamListConfig;
pub struct StructListConfig;

impl ListConfig for ArrayListConfig {
    const START_BRACE: &'static str = "[";
    const END_BRACE: &'static str = "]";
    const PAD_CONTENTS: bool = false;
}

impl ListConfig for ParamListConfig {
    const START_BRACE: &'static str = "(";
    const END_BRACE: &'static str = ")";
    const PAD_CONTENTS: bool = false;
}

impl ListConfig for StructListConfig {
    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = true;

    fn max_single_line_contents_width(&self) -> Option<usize> {
        Some(18)
    }
}

impl<'a> AstFormatter<'a> {
    pub fn list<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
        config: C,
    ) -> FormatResult {
        self.out.token_expect(C::START_BRACE)?;
        if list.is_empty() {
            self.out.token_expect(C::END_BRACE)?;
            return Ok(());
        }
        self.fallback_chain("list")
            .next("single line", |this| {
                this.list_single_line(list, &format_item, &config)
            })
            .next("wrap to fit", |this| {
                this.list_wrap_to_fit(list, &format_item, &config)
            })
            .next("separate lines", |this| {
                this.list_separate_lines(list, &format_item, &config)
            })
            .result()?;
        Ok(())
    }

    fn list_single_line<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: &impl Fn(&mut Self, &T) -> FormatResult,
        config: &C,
    ) -> FormatResult {
        if C::PAD_CONTENTS {
            self.out.space()?;
        }
        let contents = |this: &mut Self| {
            let [head, tail @ ..] = list else {
                unreachable!()
            };
            format_item(this, head)?;
            for item in tail {
                this.out.token_maybe_missing(",")?;
                this.out.space()?;
                format_item(this, item)?;
            }
            Ok(())
        };
        if let Some(max_width) = config.max_single_line_contents_width() {
            self.with_width_limit_single_line(max_width, |this| contents(this))?;
        } else {
            contents(self)?;
        }
        if C::PAD_CONTENTS {
            self.out.space()?;
        }
        self.out.token_expect(C::END_BRACE)?;
        Ok(())
    }

    fn list_wrap_to_fit<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
        config: &C,
    ) -> FormatResult {
        let format_item = |this: &mut Self, item: &T| {
            this.with_width_limit_single_line(10, |this| format_item(this, item))
        };
        self.with_indent(|this| {
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
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.out.token_expect(C::END_BRACE)?;
        Ok(())
    }

    fn list_separate_lines<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
        config: &C,
    ) -> FormatResult {
        self.with_indent(|this| {
            for item in list {
                this.out.newline_indent()?;
                format_item(this, item)?;
                this.out.token_maybe_missing(",")?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.out.token_expect(C::END_BRACE)?;
        Ok(())
    }
}
