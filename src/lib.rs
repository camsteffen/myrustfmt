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
mod ast_utils;
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
    ErrorGuaranteed, FileName,
    source_map::{FilePathMapping, SourceMap},
};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast_formatter::AstFormatter;
use crate::config::Config;
use crate::constraints::Constraints;
use source_formatter::SourceFormatter;

pub fn format_file(path: impl AsRef<Path>, config: Config) -> Result<String, ErrorGuaranteed> {
    let path = path.as_ref();
    let string = fs::read_to_string(path).unwrap();
    format(&string, config, Some(path))
}

pub fn format_file_defaults(path: impl AsRef<Path>) -> Result<String, ErrorGuaranteed> {
    format_file(path, Config::default())
}

pub fn format_str_defaults(source: &str) -> Result<String, ErrorGuaranteed> {
    format_str_config(source, Config::default())
}

pub fn format_str(source: &str, max_width: u32) -> Result<String, ErrorGuaranteed> {
    format_str_config(source, Config::default().max_width(max_width))
}

pub fn format_str_config(source: &str, config: Config) -> Result<String, ErrorGuaranteed> {
    format(source, config, None)
}

pub fn format(
    source: &str,
    config: Config,
    path: Option<&Path>,
) -> Result<String, ErrorGuaranteed> {
    parse_crate(String::from(source), path, |crate_| {
        let constraints = Constraints::new(config.max_width);
        let source_formatter = SourceFormatter::new(String::from(source), constraints);
        let ast_formatter = AstFormatter::new(config, source_formatter);
        let result = ast_formatter.crate_(&crate_);
        match result {
            Ok(()) => {}
            // todo don't panic
            Err(e) => panic!("{}", e.display(source, ast_formatter.pos(), path)),
        }
        ast_formatter.finish()
    })
}

fn parse_crate<T>(
    string: String,
    path: Option<&Path>,
    f: impl FnOnce(rustc_ast::ast::Crate) -> T,
) -> Result<T, ErrorGuaranteed> {
    let source_map = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let dcx = dcx(source_map.clone());
    // todo should this be shared for a crate?
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let psess = ParseSess::with_dcx(dcx, source_map);
        let mut parser = rustc_parse::new_parser_from_source_str(
            &psess,
            match path {
                None => FileName::anon_source_code(&string),
                // todo is this actually beneficial?
                Some(path) => FileName::from(PathBuf::from(path)),
            },
            string,
        )
        .unwrap();
        let crate_ = parser.parse_crate_mod().map_err(|err| err.emit())?;
        if let Some(error) = psess.dcx().has_errors() {
            return Err(error);
        }
        Ok(f(crate_))
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
