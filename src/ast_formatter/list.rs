pub mod config;
mod overflow;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::config::{DefaultListConfig, ListConfig, ListWrapToFitConfig};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use overflow::{ListOverflow, ListOverflowNo, ListOverflowYes};
use rustc_ast::ast;

pub struct Braces {
    start: &'static str,
    end: &'static str,
    pad: bool,
}

impl Braces {
    pub const ANGLE: &'static Braces = &Braces::new("<", ">", false);
    pub const CURLY: &'static Braces = &Braces::new("{", "}", true);
    pub const CURLY_NO_PAD: &'static Braces = &Braces::new("{", "}", false);
    pub const PARENS: &'static Braces = &Braces::new("(", ")", false);
    pub const PIPE: &'static Braces = &Braces::new("|", "|", false);
    pub const SQUARE: &'static Braces = &Braces::new("[", "]", false);

    const fn new(start: &'static str, end: &'static str, pad: bool) -> Braces {
        Braces { start, end, pad }
    }
}

pub fn list<'a, 'list, Item, FormatItem>(
    braces: &'static Braces,
    list: &'list [Item],
    format_item: FormatItem,
) -> ListBuilder<'list, 'static, 'static, Item, FormatItem, DefaultListConfig, ListOverflowNo<Item>>
where
    FormatItem: Fn(&Item) -> FormatResult,
{
    ListBuilder {
        braces,
        list,
        rest: ListRest::None,
        format_item,
        tail: &Tail::NONE,
        config: &DefaultListConfig,
        overflow: ListOverflowNo::default(),
    }
}

pub struct ListBuilder<'ast, 'tail, 'config, Item, FormatItem, Config, Overflow> {
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: FormatItem,
    rest: ListRest<'ast>,
    tail: &'tail Tail,
    config: &'config Config,
    overflow: Overflow,
}

impl<'a, 'ast, 'tail, 'config, Item, FormatItem, Config, Overflow>
    ListBuilder<'ast, 'tail, 'config, Item, FormatItem, Config, Overflow>
where
    Config: ListConfig,
    FormatItem: Fn(&Item) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    pub fn config<'config_new, ConfigNew: ListConfig>(
        self,
        config: &'config_new ConfigNew,
    ) -> ListBuilder<'ast, 'tail, 'config_new, Item, FormatItem, ConfigNew, Overflow> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config,
            overflow: self.overflow,
        }
    }

    pub fn overflow(
        self,
    ) -> ListBuilder<'ast, 'tail, 'config, Item, FormatItem, Config, ListOverflowYes<Item>> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail: self.tail,
            config: self.config,
            overflow: ListOverflowYes::default(),
        }
    }

    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListBuilder { rest, ..self }
    }

    pub fn tail<'tail_new>(
        self,
        tail: &'tail_new Tail,
    ) -> ListBuilder<'ast, 'tail_new, 'config, Item, FormatItem, Config, Overflow> {
        ListBuilder {
            braces: self.braces,
            list: self.list,
            format_item: self.format_item,
            rest: self.rest,
            tail,
            config: self.config,
            overflow: self.overflow,
        }
    }

    pub fn format(self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_default)
    }

    pub fn format_single_line(self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_single_line)
    }

    pub fn format_separate_lines(self, af: &AstFormatter) -> FormatResult {
        self.do_format(af, Self::contents_separate_lines)
    }

    fn do_format(
        &self,
        af: &AstFormatter,
        contents: impl FnOnce(&Self, &AstFormatter, &Tail) -> FormatResult,
    ) -> FormatResult {
        af.out.token(self.braces.start)?;
        if self.list.is_empty() {
            af.out.token(self.braces.end)?;
            af.tail(self.tail)?;
            return Ok(());
        }
        contents(self, af, &Tail::token(self.braces.end).and(self.tail))
    }

    fn contents_default(&self, af: &AstFormatter, tail: &Tail) -> FormatResult {
        let mut fallback = af.fallback(|| self.contents_single_line(af, tail));
        if self.config.single_line_block() {
            fallback = fallback.next(|| self.contents_single_line_block(af, tail))
        }
        match Config::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                assert!(
                    matches!(self.rest, ListRest::None),
                    "rest cannot be used with wrap-to-fit"
                );
                fallback = fallback.next(|| self.contents_wrap_to_fit(af, tail, max_element_width));
            }
            ListWrapToFitConfig::No => {}
        }
        fallback
            .next(|| self.contents_separate_lines(af, tail))
            .result()
    }

    fn contents_single_line(&self, af: &AstFormatter, tail: &Tail) -> FormatResult {
        af.with_reduce_width_limit(
            self.config.single_line_reduce_max_width(af.config()),
            || {
                af.list_contents_single_line(
                    self.list,
                    &self.format_item,
                    self.rest,
                    tail,
                    self.overflow,
                    self.braces.pad,
                    self.config.single_line_max_contents_width(),
                    self.config
                        .overflow_max_first_line_contents_width(af.config()),
                )
            },
        )
    }

    fn contents_single_line_block(&self, af: &AstFormatter, tail: &Tail) -> FormatResult {
        af.list_contents_single_line_block(
            self.list,
            self.rest,
            tail,
            &self.format_item,
            self.config.single_line_max_contents_width(),
        )
    }

    fn contents_wrap_to_fit(
        &self,
        af: &AstFormatter,
        tail: &Tail,
        max_element_width: Option<usize>,
    ) -> FormatResult {
        af.list_contents_wrap_to_fit(self.list, tail, &self.format_item, max_element_width)
    }

    fn contents_separate_lines(&self, af: &AstFormatter, tail: &Tail) -> FormatResult {
        af.list_contents_separate_lines(self.list, &self.format_item, self.rest, tail)
    }
}

#[derive(Clone, Copy)]
pub enum ListRest<'a> {
    None,
    Rest,
    Base(&'a ast::Expr),
}

impl From<ast::PatFieldsRest> for ListRest<'static> {
    fn from(rest: ast::PatFieldsRest) -> Self {
        match rest {
            ast::PatFieldsRest::None => ListRest::None,
            ast::PatFieldsRest::Rest => ListRest::Rest,
        }
    }
}

impl<'a> From<&'a ast::StructRest> for ListRest<'a> {
    fn from(rest: &'a ast::StructRest) -> Self {
        match rest {
            ast::StructRest::None => ListRest::None,
            ast::StructRest::Rest(_) => ListRest::Rest,
            ast::StructRest::Base(expr) => ListRest::Base(expr),
        }
    }
}

impl<'a> AstFormatter {
    /* [item, item, item] */
    fn list_contents_single_line<Item, Overflow: ListOverflow<Item = Item>>(
        &self,
        list: &[Item],
        format_item: impl Fn(&Item) -> FormatResult,
        rest: ListRest<'_>,
        tail: &Tail,
        _overflow: Overflow,
        pad: bool,
        max_width: Option<usize>,
        max_width_overflow: Option<usize>,
    ) -> FormatResult {
        if pad {
            self.out.space()?;
        }

        let (last, until_last) = list.split_last().unwrap();

        let last_can_overflow = Overflow::can_overflow(self, last, list.len() == 1);
        let can_overflow = matches!(rest, ListRest::None)
            && self.allow_multiline_overflow.get()
            && last_can_overflow;

        let format = || {
            let start = self.out.last_line_len();
            self.with_single_line(|| -> FormatResult {
                for item in until_last {
                    format_item(item)?;
                    self.out.token_maybe_missing(",")?;
                    self.out.space()?;
                }
                Ok(())
            })?;
            if can_overflow {
                self.fallback(|| self.with_single_line(|| format_item(last)))
                    .next(|| {
                        self.with_width_limit_from_start_first_line_opt(
                            start,
                            max_width_overflow,
                            || Overflow::format_overflow(self, last, list.len() == 1),
                        )
                    })
                    .result()?;
            } else {
                self.with_single_line(|| format_item(last))?;
            }
            if matches!(rest, ListRest::None) {
                self.out.skip_token_if_present(",")?;
            } else {
                self.with_single_line(|| -> FormatResult {
                    self.out.token_maybe_missing(",")?;
                    self.out.space()?;
                    self.out.token("..")?;
                    if let ListRest::Base(expr) = rest {
                        self.expr(expr)?;
                    }
                    Ok(())
                })?;
            }
            FormatResult::Ok(())
        };
        // let format = || self.with_single_line(format);
        self.with_width_limit_first_line_opt(max_width, format)?;
        if pad {
            self.out.space()?;
        }
        self.tail(tail)?;
        Ok(())
    }

    /*
    [
        item, item
    ]
     */
    fn list_contents_single_line_block<Item>(
        &self,
        list: &[Item],
        rest: ListRest<'_>,
        tail: &Tail,
        format_item: impl Fn(&Item) -> FormatResult,
        max_width: Option<usize>,
    ) -> FormatResult {
        // single line block only exists for a specific case of rustfmt compatibility
        assert!(
            matches!(rest, ListRest::Rest),
            "single line block list can only be used with ListRest::Rest"
        );
        let (last, until_last) = list.split_last().unwrap();
        self.indented(|| {
            self.out.newline_indent()?;
            self.with_single_line(|| {
                self.with_width_limit_opt(max_width, || {
                    for item in until_last {
                        format_item(item)?;
                        self.out.token_maybe_missing(",")?;
                        self.out.space()?;
                    }
                    format_item(last)?;
                    Ok(())
                })?;
                self.out.token_maybe_missing(",")?;
                self.out.space()?;
                self.out.token("..")?;
                Ok(())
            })
        })?;
        self.out.newline_indent()?;
        self.tail(tail)?;
        Ok(())
    }

    /*
    [
        item, item, item,
        item,
    ]
    */
    fn list_contents_wrap_to_fit<T>(
        &self,
        list: &[T],
        tail: &Tail,
        format_item: impl Fn(&T) -> FormatResult,
        max_element_width: Option<usize>,
    ) -> FormatResult {
        let format_item = |item| match max_element_width {
            Some(max_width) => self.with_width_limit_single_line(max_width, || format_item(item)),
            None => format_item(item),
        };
        self.indented(|| {
            self.out.newline_indent()?;
            let (first, rest) = list.split_first().unwrap();
            format_item(first)?;
            self.out.token_maybe_missing(",")?;
            for item in rest {
                let item_comma = || -> FormatResult {
                    format_item(item)?;
                    self.out.token_maybe_missing(",")?;
                    Ok(())
                };
                self.fallback(|| {
                    self.out.space()?;
                    item_comma()?;
                    Ok(())
                })
                .next(|| {
                    self.out.newline_indent()?;
                    item_comma()?;
                    Ok(())
                })
                .result()?;
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.tail(tail)?;
        Ok(())
    }

    /*
    [
        item,
        item,
        item,
    ]
    */
    fn list_contents_separate_lines<T>(
        &self,
        list: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        rest: ListRest<'_>,
        tail: &Tail,
    ) -> FormatResult {
        self.indented(|| {
            for item in list {
                self.out.newline_indent()?;
                format_item(item)?;
                self.out.token_maybe_missing(",")?;
            }
            if !matches!(rest, ListRest::None) {
                self.out.newline_indent()?;
                self.out.token("..")?;
                if let ListRest::Base(expr) = rest {
                    self.expr(expr)?;
                }
            }
            Ok(())
        })?;
        self.out.newline_indent()?;
        self.tail(tail)?;
        Ok(())
    }
}
