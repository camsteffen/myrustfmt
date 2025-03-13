#![feature(rustc_private)]

use myrustfmt::format_str;
use myrustfmt::config::Config;

// https://github.com/rust-lang/rustfmt/issues/4867
#[test]
fn slow_test() {
    let source = r#"
mod modA {
    mod modB {
        mod modC {
            mod modD {
                mod modE {
                    fn func() {
                        state . rule (Rule :: myrule , | state | { state . sequence (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . sequence (| state | { state . optional (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) . and_then (| state | { state . repeat (| state | { state . sequence (| state | { super :: hidden :: skip (state) . and_then (| state | { state . sequence (| state | { state . match_string ("abc") . and_then (| state | { super :: hidden :: skip (state) }) . and_then (| state | { state . match_string ("def") }) }) }) }) }) }) }) }) }) }) });
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(
        format_str(source, Config::default().max_width(800)).unwrap().formatted,
        r#"
mod modA {
    mod modB {
        mod modC {
            mod modD {
                mod modE {
                    fn func() {
                        state.rule(Rule::myrule, |state| {
                            state.sequence(|state| {
                                state
                                    .sequence(|state| {
                                        state
                                            .match_string("abc")
                                            .and_then(|state| super::hidden::skip(state))
                                            .and_then(|state| state.match_string("def"))
                                    })
                                    .and_then(|state| super::hidden::skip(state))
                                    .and_then(|state| {
                                        state.sequence(|state| {
                                            state.optional(|state| {
                                                state
                                                    .sequence(|state| {
                                                        state
                                                            .match_string("abc")
                                                            .and_then(|state| super::hidden::skip(state))
                                                            .and_then(|state| state.match_string("def"))
                                                    })
                                                    .and_then(|state| {
                                                        state.repeat(|state| {
                                                            state.sequence(|state| {
                                                                super::hidden::skip(state).and_then(|state| {
                                                                    state.sequence(|state| {
                                                                        state
                                                                            .match_string("abc")
                                                                            .and_then(|state| super::hidden::skip(state))
                                                                            .and_then(|state| state.match_string("def"))
                                                                    })
                                                                })
                                                            })
                                                        })
                                                    })
                                            })
                                        })
                                    })
                            })
                        });
                    }
                }
            }
        }
    }
}
"#
        .trim_start()
    );
}
