use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use std::marker::PhantomData;

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

pub trait ListConfig {
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

pub struct DefaultListConfig;
impl ListConfig for DefaultListConfig {}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<usize> },
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.array_width)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: Some(RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold),
        }
    }
}

pub struct ParamListConfig {
    pub single_line_max_contents_width: Option<usize>,
}
impl ListConfig for ParamListConfig {
    fn single_line_max_contents_width(&self) -> Option<usize> {
        self.single_line_max_contents_width
    }
}

pub fn struct_field_list_config(
    single_line_block: bool,
    single_line_max_contents_width: usize,
) -> impl ListConfig {
    pub struct StructFieldListConfig {
        single_line_block: bool,
        single_line_max_contents_width: usize,
    }
    impl ListConfig for StructFieldListConfig {
        fn single_line_block(&self) -> bool {
            self.single_line_block
        }

        fn single_line_max_contents_width(&self) -> Option<usize> {
            Some(self.single_line_max_contents_width)
        }
    }
    StructFieldListConfig {
        single_line_block,
        single_line_max_contents_width,
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
        tail: Tail::NONE,
        config: &DefaultListConfig,
        overflow: ListOverflowNo(PhantomData),
    }
}

pub struct ListBuilder<'ast, 'tail, 'config, Item, FormatItem, Config, Overflow> {
    braces: &'static Braces,
    list: &'ast [Item],
    format_item: FormatItem,
    rest: ListRest<'ast>,
    tail: Tail<'tail>,
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
            overflow: ListOverflowYes(PhantomData),
        }
    }

    pub fn rest(self, rest: ListRest<'ast>) -> Self {
        ListBuilder { rest, ..self }
    }

    pub fn tail<'tail_new>(
        self,
        tail: Tail<'tail_new>,
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

    pub fn format(self, this: &AstFormatter) -> FormatResult {
        this.format_list(
            self.braces,
            self.list.is_empty(),
            |tail| {
                this.list_contents(
                    self.list,
                    self.format_item,
                    self.rest,
                    tail,
                    self.overflow,
                    self.config,
                    self.braces.pad,
                )
            },
            self.tail,
        )
    }

    pub fn format_single_line(self, this: &AstFormatter) -> FormatResult {
        this.format_list(
            self.braces,
            self.list.is_empty(),
            |tail| {
                this.list_contents_single_line(
                    self.list,
                    self.format_item,
                    self.rest,
                    tail,
                    self.overflow,
                    self.braces.pad,
                    self.config.single_line_max_contents_width(),
                )
            },
            self.tail,
        )
    }

    pub fn format_separate_lines(self, this: &AstFormatter) -> FormatResult {
        this.format_list(
            self.braces,
            self.list.is_empty(),
            |tail| this.list_contents_separate_lines(self.list, self.format_item, self.rest, tail),
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
    fn format_list<'b, 'c>(
        &self,
        braces: &'static Braces,
        is_empty: bool,
        contents: impl FnOnce(Tail) -> FormatResult + 'b,
        end: Tail<'_>,
    ) -> FormatResult {
        self.out.token_expect(braces.start)?;
        if is_empty {
            self.out.token_expect(braces.end)?;
            self.tail(end)?;
            return Ok(());
        }
        contents(Tail::new(&move || {
            self.out.token_expect(braces.end)?;
            self.tail(end)?;
            Ok(())
        }))
    }

    fn list_contents<T, Config>(
        &self,
        list: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        rest: ListRest<'_>,
        tail: Tail<'_>,
        overflow: impl ListOverflow<Item = T>,
        config: &Config,
        pad: bool,
    ) -> FormatResult
    where
        Config: ListConfig,
    {
        let mut fallback = self.fallback(|| {
            self.list_contents_single_line(
                list,
                &format_item,
                rest,
                tail,
                overflow,
                pad,
                config.single_line_max_contents_width(),
            )
        });
        if config.single_line_block() {
            fallback = fallback.next(|| {
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
                fallback = fallback.next(|| {
                    self.list_contents_wrap_to_fit(list, tail, &format_item, max_element_width)
                });
            }
            ListWrapToFitConfig::No => {}
        }
        fallback
            .next(|| self.list_contents_separate_lines(list, format_item, rest, tail))
            .result()
    }

    /* [item, item, item] */
    fn list_contents_single_line<Item, Overflow: ListOverflow<Item = Item>>(
        &self,
        list: &[Item],
        format_item: impl Fn(&Item) -> FormatResult,
        rest: ListRest<'_>,
        tail: Tail,
        _overflow: Overflow,
        pad: bool,
        max_width: Option<usize>,
    ) -> FormatResult {
        if pad {
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
            if pad {
                self.out.space()?;
            }
            Ok(())
        };
        let format = || self.with_single_line(format);
        if let Some(max_width) = max_width {
            self.with_width_limit_first_line(max_width, format)?;
        } else {
            format()?;
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
