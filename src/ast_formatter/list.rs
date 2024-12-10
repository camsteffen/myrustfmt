use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

pub trait ListConfig {
    const START_BRACE: &'static str;
    const END_BRACE: &'static str;
    const PAD_CONTENTS: bool;

    fn single_line_max_contents_width() -> Option<usize> {
        None
    }

    fn wrap_to_fit() -> ListWrapToFitConfig;
}

enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: usize },
}

pub struct ArrayListConfig;
pub struct ParamListConfig;
pub struct StructListConfig;

impl ListConfig for ArrayListConfig {
    const START_BRACE: &'static str = "[";
    const END_BRACE: &'static str = "]";
    const PAD_CONTENTS: bool = false;

    fn single_line_max_contents_width() -> Option<usize> {
        // array_width in rustfmt
        Some(60)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        // short_array_element_width_threshold in rustfmt
        ListWrapToFitConfig::Yes {
            max_element_width: 10,
        }
    }
}

impl ListConfig for ParamListConfig {
    const START_BRACE: &'static str = "(";
    const END_BRACE: &'static str = ")";
    const PAD_CONTENTS: bool = false;

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

impl ListConfig for StructListConfig {
    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = true;

    fn single_line_max_contents_width() -> Option<usize> {
        // struct_lit_width in rustfmt
        Some(18)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

impl<'a> AstFormatter<'a> {
    pub fn list<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult + Copy,
        config: C,
    ) -> FormatResult {
        self.out.token_expect(C::START_BRACE)?;
        if list.is_empty() {
            self.out.token_expect(C::END_BRACE)?;
            return Ok(());
        }
        let mut fallback = self.fallback_chain("list").next("single line", |this| {
            this.list_single_line(list, &format_item, &config)
        });
        match C::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                fallback = fallback.next("wrap to fit", move |this| {
                    this.list_wrap_to_fit(list, &format_item, max_element_width)
                });
            }
            ListWrapToFitConfig::No => {}
        }
        fallback
            .next("separate lines", |this| {
                this.list_separate_lines(list, &format_item, &config)
            })
            .finally(|this| this.out.token_expect(C::END_BRACE))
            .execute(self)?;
        Ok(())
    }

    fn list_single_line<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: &impl Fn(&mut Self, &T) -> FormatResult,
        _config: &C,
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
        if let Some(max_width) = C::single_line_max_contents_width() {
            self.with_width_limit_single_line(max_width, |this| contents(this))?;
        } else {
            contents(self)?;
        }
        if C::PAD_CONTENTS {
            self.out.space()?;
        }
        Ok(())
    }

    fn list_wrap_to_fit<T>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
        max_element_width: usize,
    ) -> FormatResult {
        let format_item = |this: &mut Self, item: &T| {
            this.with_width_limit_single_line(max_element_width, |this| format_item(this, item))
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
                    .next("same line", |this| this.out.space())
                    .next("wrap", |this| this.out.newline_indent())
                    .finally(|this| {
                        format_item(this, item)?;
                        this.out.token_maybe_missing(",")?;
                        Ok(())
                    })
                    .execute(this)?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        Ok(())
    }

    fn list_separate_lines<T, C: ListConfig>(
        &mut self,
        list: &[T],
        format_item: impl Fn(&mut Self, &T) -> FormatResult,
        _config: &C,
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
        Ok(())
    }
}
