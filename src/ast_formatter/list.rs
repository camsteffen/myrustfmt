use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::marker::PhantomData;

pub trait ListConfig {
    const START_BRACE: &'static str;
    const END_BRACE: &'static str;
    const PAD_CONTENTS: bool;

    fn single_line_block(&self) -> bool {
        false
    }

    fn single_line_max_contents_width(&self) -> Option<usize> {
        None
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<usize> },
}

pub struct AngleBracketedListConfig;
impl ListConfig for AngleBracketedListConfig {
    const START_BRACE: &'static str = "<";
    const END_BRACE: &'static str = ">";
    const PAD_CONTENTS: bool = false;
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    const START_BRACE: &'static str = "[";
    const END_BRACE: &'static str = "]";
    const PAD_CONTENTS: bool = false;

    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.array_width)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        // short_array_element_width_threshold in rustfmt
        ListWrapToFitConfig::Yes {
            max_element_width: Some(10),
        }
    }
}

pub fn param_list_config(single_line_max_contents_width: Option<usize>) -> impl ListConfig {
    struct ParamListConfig {
        single_line_max_contents_width: Option<usize>,
    }
    impl ListConfig for ParamListConfig {
        const START_BRACE: &'static str = "(";
        const END_BRACE: &'static str = ")";
        const PAD_CONTENTS: bool = false;

        fn single_line_max_contents_width(&self) -> Option<usize> {
            self.single_line_max_contents_width
        }
    }
    ParamListConfig {
        single_line_max_contents_width,
    }
}

pub fn struct_field_list_config(single_line_block: bool) -> impl ListConfig {
    pub struct StructFieldListConfig {
        single_line_block: bool,
    }
    impl ListConfig for StructFieldListConfig {
        const START_BRACE: &'static str = "{";
        const END_BRACE: &'static str = "}";
        const PAD_CONTENTS: bool = true;

        fn single_line_block(&self) -> bool {
            self.single_line_block
        }

        fn single_line_max_contents_width(&self) -> Option<usize> {
            Some(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
        }
    }
    StructFieldListConfig { single_line_block }
}

pub struct StructFieldListConfig;

impl ListConfig for StructFieldListConfig {
    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = true;

    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.struct_lit_width)
    }
}

pub trait ListOverflow {
    type Item;

    fn format_if_overflow(
        _ast_formatter: &AstFormatter,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> Option<FormatResult> {
        None
    }
}

pub struct ListOverflowNo<T>(PhantomData<T>);
pub struct ListOverflowYes<T>(PhantomData<T>);

impl<T> ListOverflow for ListOverflowNo<T> {
    type Item = T;

    fn format_if_overflow(
        _ast_formatter: &AstFormatter,
        _item: &Self::Item,
        _is_only_item: bool,
    ) -> Option<FormatResult> {
        None
    }
}

impl<T: Overflow> ListOverflow for ListOverflowYes<T> {
    type Item = T;

    fn format_if_overflow(
        ast_formatter: &AstFormatter,
        item: &Self::Item,
        is_only_item: bool,
    ) -> Option<FormatResult> {
        Overflow::format_if_overflow(ast_formatter, item, is_only_item)
    }
}

pub fn list<'a, 'list, Item, FormatItem, Config>(
    list: &'list [Item],
    format_item: FormatItem,
    config: Config,
) -> ListBuilder<'list, 'static, Item, FormatItem, Config, ListOverflowNo<Item>>
where
    Config: ListConfig,
    FormatItem: Fn(&Item) -> FormatResult,
{
    ListBuilder {
        list,
        rest: ListRest::None,
        format_item,
        config,
        tail: Tail::NONE,
        overflow: ListOverflowNo(PhantomData),
    }
}

pub struct ListBuilder<'ast, 'tail, Item, FormatItem, Config, Overflow> {
    list: &'ast [Item],
    rest: ListRest<'ast>,
    format_item: FormatItem,
    config: Config,
    tail: Tail<'tail>,
    overflow: Overflow,
}

impl<'a, 'ast, 'tail, Item, FormatItem, Config, Overflow>
    ListBuilder<'ast, 'tail, Item, FormatItem, Config, Overflow>
where
    Config: ListConfig,
    FormatItem: Fn(&Item) -> FormatResult,
    Overflow: ListOverflow<Item = Item>,
{
    pub fn overflow(
        self,
    ) -> ListBuilder<'ast, 'tail, Item, FormatItem, Config, ListOverflowYes<Item>> {
        ListBuilder {
            list: self.list,
            rest: ListRest::None,
            format_item: self.format_item,
            config: self.config,
            tail: self.tail,
            overflow: ListOverflowYes(PhantomData),
        }
    }

    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListBuilder { rest, ..self }
    }

    pub fn tail<'tail_new>(
        self,
        tail: Tail<'tail_new>,
    ) -> ListBuilder<'ast, 'tail_new, Item, FormatItem, Config, Overflow> {
        ListBuilder {
            list: self.list,
            rest: self.rest,
            format_item: self.format_item,
            config: self.config,
            overflow: self.overflow,
            tail,
        }
    }

    pub fn format(self, this: &AstFormatter) -> FormatResult {
        this.format_list(
            Config::START_BRACE,
            Config::END_BRACE,
            self.list.is_empty(),
            |tail| {
                this.list_contents(
                    self.list,
                    self.rest,
                    self.format_item,
                    self.overflow,
                    self.config,
                    tail,
                )
            },
            self.tail,
        )
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
    pub fn list_separate_lines<T>(
        &self,
        list: &[T],
        start_brace: &'static str,
        end_brace: &'static str,
        format_item: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        self.format_list(
            start_brace,
            end_brace,
            list.is_empty(),
            |tail| self.list_contents_separate_lines(list, ListRest::None, format_item, tail),
            Tail::NONE,
        )
    }

    fn format_list<'b, 'c>(
        &self,
        start_brace: &'static str,
        end_brace: &'static str,
        is_empty: bool,
        non_empty: impl FnOnce(Tail) -> FormatResult + 'b,
        end: Tail<'_>,
    ) -> FormatResult {
        self.out.token_expect(start_brace)?;
        if is_empty {
            self.out.token_expect(end_brace)?;
            self.tail(end)?;
            return Ok(());
        }
        non_empty(Tail::new(&move || {
            self.out.token_expect(end_brace)?;
            self.tail(end)?;
            Ok(())
        }))
    }

    fn list_contents<T, Config>(
        &self,
        list: &[T],
        rest: ListRest<'_>,
        format_item: impl Fn(&T) -> FormatResult,
        overflow: impl ListOverflow<Item = T>,
        config: Config,
        tail: Tail<'_>,
    ) -> FormatResult
    where
        Config: ListConfig,
    {
        let mut fallback = self.fallback(|| {
            self.list_contents_single_line(
                list,
                rest,
                tail,
                &format_item,
                overflow,
                Config::PAD_CONTENTS,
                config.single_line_max_contents_width(),
            )
        });
        if config.single_line_block() {
            fallback = fallback.next(self, || {
                self.list_contents_single_line_block(
                    list,
                    rest,
                    tail,
                    &format_item,
                    config.single_line_max_contents_width(),
                )
            })
        }
        match Config::wrap_to_fit() {
            ListWrapToFitConfig::Yes { max_element_width } => {
                assert!(
                    matches!(rest, ListRest::None),
                    "rest cannot be used with wrap-to-fit"
                );
                fallback = fallback.next(self, || {
                    self.list_contents_wrap_to_fit(list, tail, &format_item, max_element_width)
                });
            }
            ListWrapToFitConfig::No => {}
        }
        fallback
            .next(self, || {
                self.list_contents_separate_lines(list, rest, format_item, tail)
            })
            .result()
    }

    /* [item, item, item] */
    fn list_contents_single_line<Item, Overflow: ListOverflow<Item = Item>>(
        &self,
        list: &[Item],
        rest: ListRest<'_>,
        tail: Tail,
        format_item: impl Fn(&Item) -> FormatResult,
        _overflow: Overflow,
        pad_contents: bool,
        max_width: Option<usize>,
    ) -> FormatResult {
        if pad_contents {
            self.out.space()?;
        }
        let (last, until_last) = list.split_last().unwrap();
        let format = || {
            for item in until_last {
                format_item(item)?;
                self.out.token_maybe_missing(",")?;
                self.out.space()?;
            }
            let overflow_result = if matches!(rest, ListRest::None) && self.allow_overflow.get() {
                Overflow::format_if_overflow(self, last, list.len() == 1)
            } else {
                None
            };
            if let Some(result) = overflow_result {
                result?;
            } else {
                format_item(last)?;
            };
            if matches!(rest, ListRest::None) {
                self.out.skip_token_if_present(",")?;
            } else {
                self.out.token_maybe_missing(",")?;
                self.out.space()?;
                self.out.token_expect("..")?;
                if let ListRest::Base(expr) = rest {
                    self.expr(expr)?;
                }
            }
            if pad_contents {
                self.out.space()?;
            }
            self.tail(tail)?;
            Ok(())
        };
        let format = || self.with_single_line(format);
        if let Some(max_width) = max_width {
            self.with_width_limit_first_line(max_width, format)?;
        } else {
            format()?;
        }
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
        tail: Tail,
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
                self.out.token_expect("..")?;
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
        tail: Tail,
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
                let item_comma = || {
                    format_item(item)?;
                    self.out.token_maybe_missing(",")?;
                    Ok(())
                };
                self.fallback(|| {
                    self.out.space()?;
                    item_comma()?;
                    Ok(())
                })
                .next(self, || {
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
        rest: ListRest<'_>,
        format_item: impl Fn(&T) -> FormatResult,
        tail: Tail<'_>,
    ) -> FormatResult {
        self.indented(|| {
            for item in list {
                self.out.newline_indent()?;
                format_item(item)?;
                self.out.token_maybe_missing(",")?;
            }
            if !matches!(rest, ListRest::None) {
                self.out.newline_indent()?;
                self.out.token_expect("..")?;
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

trait OverflowHandler {
    type Result;

    const FORMATTING: bool;

    fn no_overflow() -> Self::Result;
    fn overflows(format: impl FnOnce() -> FormatResult) -> Self::Result;
}

struct CheckIfOverflow;
struct OverflowDoFormat;

impl OverflowHandler for CheckIfOverflow {
    type Result = bool;

    const FORMATTING: bool = false;

    fn no_overflow() -> bool {
        false
    }

    fn overflows(_format: impl FnOnce() -> FormatResult) -> bool {
        true
    }
}

impl OverflowHandler for OverflowDoFormat {
    type Result = FormatResult;

    const FORMATTING: bool = true;

    fn no_overflow() -> FormatResult {
        unreachable!()
    }

    fn overflows(format: impl FnOnce() -> FormatResult) -> FormatResult {
        format()
    }
}

trait Overflow {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        t: &Self,
        is_only_list_item: bool,
    ) -> H::Result;

    fn check_if_overflows(this: &AstFormatter, t: &Self, is_only_list_item: bool) -> bool {
        Self::format_or_check_if_overflow::<CheckIfOverflow>(this, t, is_only_list_item)
    }

    fn format(this: &AstFormatter, t: &Self, is_only_list_item: bool) -> FormatResult {
        Self::format_or_check_if_overflow::<OverflowDoFormat>(this, t, is_only_list_item)
    }

    fn format_if_overflow(
        this: &AstFormatter,
        t: &Self,
        is_only_list_item: bool,
    ) -> Option<FormatResult> {
        if Self::check_if_overflows(this, t, is_only_list_item) {
            Some(Self::format(this, t, is_only_list_item))
        } else {
            None
        }
    }
}

impl Overflow for ast::Expr {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        expr: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        if !expr.attrs.is_empty() {
            return H::no_overflow();
        }
        match expr.kind {
            // block-like
            ast::ExprKind::Block(..) | ast::ExprKind::Gen(..) => H::overflows(|| this.expr(expr)),
            ast::ExprKind::Closure(ref closure) => {
                H::overflows(|| this.closure(closure, true, Tail::NONE))
            }
            // control flow
            // | ast::ExprKind::ForLoop { .. }
            // | ast::ExprKind::If(..)
            // | ast::ExprKind::Loop(..)
            // | ast::ExprKind::Match(..)
            // | ast::ExprKind::While(..)
            // list
            // | ast::ExprKind::Array(..)
            // | ast::ExprKind::Call(..)
            // | ast::ExprKind::MacCall(..)
            // | ast::ExprKind::Struct(..)
            // | ast::ExprKind::Tup(..) if is_only_list_item => H::overflows(|| this.expr(expr, Tail::None)),
            // | ast::ExprKind::MethodCall(..) if is_only_list_item => H::overflows(|| this.dot_chain(expr, Tail::NONE, true)),
            // prefix
            ast::ExprKind::AddrOf(borrow_kind, mutability, ref target)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                H::overflows(|| {
                    this.addr_of(borrow_kind, mutability, expr)?;
                    Overflow::format(this, target, is_only_list_item)
                })
            }
            ast::ExprKind::Cast(ref target, _)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Try(ref target)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            ast::ExprKind::Unary(_, ref target)
                if H::FORMATTING
                    || Overflow::check_if_overflows(this, target, is_only_list_item) =>
            {
                todo!()
            }
            _ => H::no_overflow(),
        }
    }
}

impl Overflow for ast::MetaItemInner {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        item: &Self,
        _is_only_list_item: bool,
    ) -> H::Result {
        match item {
            ast::MetaItemInner::Lit(..) => H::overflows(|| todo!()),
            ast::MetaItemInner::MetaItem(meta_item) => {
                if matches!(meta_item.kind, ast::MetaItemKind::Word) {
                    H::overflows(|| this.meta_item(meta_item))
                } else {
                    H::no_overflow()
                }
            }
        }
    }
}

impl<T: Overflow> Overflow for P<T> {
    fn format_or_check_if_overflow<H: OverflowHandler>(
        this: &AstFormatter,
        t: &Self,
        is_only_list_item: bool,
    ) -> H::Result {
        <T as Overflow>::format_or_check_if_overflow::<H>(this, t, is_only_list_item)
    }
}
