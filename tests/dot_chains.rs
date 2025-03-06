#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::{format_str_config, format_str_defaults};
use tracing_test::traced_test;

#[traced_test]
#[test]
fn dot_chain_wrap_instead_of_overflow() {
    let source = r#"
fn test() {
    if self.constraints_mut().newline_budget.is_some_and(|h| h <= newlines) {
        return f(self);
    }
}"#;
    assert_eq!(
        format_str_defaults(source)
            .unwrap()
            .expect_no_errors(),
        r#"
fn test() {
    if self
        .constraints_mut()
        .newline_budget
        .is_some_and(|h| h <= newlines)
    {
        return f(self);
    }
}
"#
        .trim_start()
    );
}

#[test]
fn overflow_single_item_chain() {
    let source = r#"
fn test() {
    self.fallback(|| { let x; })
}"#;
    assert_eq!(
        format_str_config(source, Config::default().max_width(22))
            .unwrap()
            .expect_no_errors(),
        r#"
fn test() {
    self.fallback(|| {
        let x;
    })
}
"#
        .trim_start()
    );
}

#[test]
fn overflow_first_line_with_chaining_after() {
    let source = r#"
fn test() {
    self.fallback(|| {
        let x;
    })
    .next(|| {
        let x;
    });
}"#;
    assert_eq!(
        format_str_config(source, Config::default())
            .unwrap()
            .expect_no_errors(),
        r#"
fn test() {
    self.fallback(|| {
        let x;
    })
    .next(|| {
        let x;
    });
}
"#
        .trim_start()
    );
}

#[test]
fn dot_chain_single_child_can_exceed_chain_width() {
    assert_eq!(
        format_str_defaults(
            "fn test() { fallback = fallback.next(|| self.contents_wrap_to_fit(af, tail, max_element_width)); }",
        )
            .unwrap().expect_no_errors(),
        "
fn test() {
    fallback = fallback.next(|| self.contents_wrap_to_fit(af, tail, max_element_width));
}
"
            .trim_start()
    );
}

#[traced_test]
#[test]
fn dot_chain_over_chain_width() {
    assert_eq!(
        format_str_defaults(
            "fn test() { self.config.overflow_max_first_line_contents_width(affconfig()); }",
        )
        .unwrap()
        .expect_no_errors(),
        "
fn test() {
    self.config
        .overflow_max_first_line_contents_width(affconfig());
}
"
        .trim_start()
    );
}

#[traced_test]
#[test]
fn first_item_within_margin_may_exceed_width() {
    assert_eq!(
        format_str_defaults(
            "fn test() { self.falasdfasdflback(|| self.with_single_line(|| format_item(last))) .next(|| { let x; }) .result()?; }",
        )
            .unwrap().expect_no_errors(),
        "
fn test() {
    self.falasdfasdflback(|| self.with_single_line(|| format_item(last)))
        .next(|| {
            let x;
        })
        .result()?;
}
"
            .trim_start()
    );
}

#[test]
fn overflow_last_item() {
    assert_eq!(
        format_str_defaults(
            "fn test() { chain.iter().try_for_each(|(op, expr)| -> FormatResult { let x; })?; }",
        )
        .unwrap()
        .expect_no_errors(),
        "
fn test() {
    chain.iter().try_for_each(|(op, expr)| -> FormatResult {
        let x;
    })?;
}
"
        .trim_start()
    );
}

#[test]
fn fn_call_width_exceeded_in_chain() {
    assert_eq!(
        format_str_defaults(
            "fn test() { list(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) .config(aaaaaaaaaaaaaaaaaaaaaaaa( false, bbbbbbbbbbbbbbbbbbbbbbbbbbbb, )) .format()?; }",
        )
            .unwrap().expect_no_errors(),
        "
fn test() {
    list(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)
        .config(aaaaaaaaaaaaaaaaaaaaaaaa(
            false,
            bbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ))
        .format()?;
}
"
            .trim_start()
    );
}
