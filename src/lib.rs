#![feature(rustc_private)]

// these crates are loaded from the sysroot, so they need extern crate.
extern crate rustc_ast;
// extern crate rustc_ast_pretty;
// extern crate rustc_builtin_macros;
// extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
// extern crate rustc_expand;
extern crate rustc_lexer;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;

pub mod ast_formatter;
mod ast_utils;
pub mod config;
pub mod constraint_writer;
mod constraints;
mod error;
mod error_emitter;
mod parse;
mod rustfmt_config_defaults;
pub mod source_formatter;
mod source_reader;
mod util;

use crate::ast_formatter::{AstFormatter, FormatCrateResult};
use crate::config::Config;
use crate::parse::parse_crate;
use rustc_span::ErrorGuaranteed;
use rustc_span::edition::Edition;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum CrateSource<'a> {
    File(&'a Path),
    Source(&'a str),
}

impl<'a> CrateSource<'a> {
    pub fn path(self) -> Option<&'a Path> {
        match self {
            CrateSource::File(path) => Some(path),
            CrateSource::Source(_) => None,
        }
    }
}

pub fn format_file(path: &Path, config: Config) -> Result<FormatCrateResult, ErrorGuaranteed> {
    format(CrateSource::File(path), config)
}

pub fn format_file_defaults(path: &Path) -> Result<FormatCrateResult, ErrorGuaranteed> {
    format_file(path, Config::default())
}

pub fn format_str_defaults(source: &str) -> Result<FormatCrateResult, ErrorGuaranteed> {
    format_str_config(source, Config::default())
}

pub fn format_str(source: &str, max_width: u32) -> Result<FormatCrateResult, ErrorGuaranteed> {
    format_str_config(source, Config::default().max_width(max_width))
}

pub fn format_str_config(
    source: &str,
    config: Config,
) -> Result<FormatCrateResult, ErrorGuaranteed> {
    format(CrateSource::Source(source), config)
}

pub fn format(
    crate_source: CrateSource<'_>,
    config: Config,
) -> Result<FormatCrateResult, ErrorGuaranteed> {
    // todo should this be shared across crates?
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let (crate_, source) = parse_crate(crate_source)?;
        let path = crate_source.path().map(Path::to_path_buf);
        let ast_formatter = AstFormatter::new(source, path, config);
        Ok(ast_formatter.crate_(&crate_))
    })
}
