#![feature(rustc_private)]

// N.B. these crates are loaded from the sysroot, so they need extern crate.
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_builtin_macros;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_lexer;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate thin_vec;

// Necessary to pull in object code as the rest of the rustc crates are shipped only as rmeta
// files.
#[allow(unused_extern_crates)]
extern crate rustc_driver;

pub mod ast_formatter;
pub mod config;
pub mod constraint_writer;
mod constraints;
mod error;
mod rustfmt_config_defaults;
pub mod source_formatter;
mod source_reader;

use rustc_data_structures::sync::Lrc;
use rustc_errors::emitter::{HumanEmitter, stderr_destination};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::{
    FileName,
    source_map::{FilePathMapping, SourceMap},
};
use std::fs;
use std::path::Path;

use crate::ast_formatter::AstFormatter;
use crate::config::Config;
use crate::constraints::Constraints;
use source_formatter::SourceFormatter;

pub fn format_file(path: impl AsRef<Path>) -> String {
    let string = fs::read_to_string(path).unwrap();
    format_str_config(&string, Config::default())
}

pub fn format_str_defaults(source: &str) -> String {
    format_str_config(source, Config::default())
}

pub fn format_str(source: &str, max_width: usize) -> String {
    format_str_config(source, Config::default().max_width(max_width))
}

pub fn format_str_config(source: &str, config: Config) -> String {
    parse_ast_then(String::from(source), |crate_| {
        let constraints = Constraints::new(config.max_width);
        let source_formatter = SourceFormatter::new(String::from(source), constraints);
        let ast_formatter = AstFormatter::new(config, source_formatter);
        match ast_formatter.crate_(&crate_) {
            Ok(()) => {}
            Err(e) => panic!("{}", e.display(source)),
        }
        ast_formatter.finish()
    })
}

fn parse_ast_then<T>(string: String, f: impl FnOnce(rustc_ast::ast::Crate) -> T) -> T {
    let source_map = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let dcx = dcx(source_map.clone());
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let psess = ParseSess::with_dcx(dcx, source_map);
        let mut parser = rustc_parse::new_parser_from_source_str(
            &psess,
            FileName::anon_source_code(&string),
            string,
        )
        .unwrap();
        let crate_ = parser.parse_crate_mod().unwrap_or_else(|err| {
            err.emit();
            panic!("ur done");
        });
        f(crate_)
    })
}

fn dcx(source_map: Lrc<SourceMap>) -> DiagCtxt {
    let fallback_bundle = rustc_errors::fallback_fluent_bundle(
        rustc_driver::DEFAULT_LOCALE_RESOURCES.to_vec(),
        false,
    );
    let emitter = Box::new(
        HumanEmitter::new(stderr_destination(ColorConfig::Auto), fallback_bundle)
            .sm(Some(source_map)),
    );

    DiagCtxt::new(emitter)
}
