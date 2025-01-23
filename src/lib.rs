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
// #[allow(unused_extern_crates)]
extern crate rustc_driver;

pub mod ast_formatter;
mod ast_utils;
pub mod config;
pub mod constraint_writer;
mod constraints;
mod error;
mod error_emitter;
mod rustfmt_config_defaults;
pub mod source_formatter;
mod source_reader;
mod util;

use crate::ast_formatter::{AstFormatter, FormatCrateResult};
use crate::config::Config;
use rustc_errors::emitter::{HumanEmitter, stderr_destination};
use rustc_errors::{ColorConfig, Diag, DiagCtxt};
use rustc_parse::parser::Parser;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::{
    ErrorGuaranteed, FileName,
    source_map::{FilePathMapping, SourceMap},
};
use std::path::Path;
use std::sync::Arc;

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
        let ast_formatter =
            AstFormatter::new(source, crate_source.path().map(Path::to_path_buf), config);
        match ast_formatter.crate_(&crate_) {
            Ok(()) => {}
            // todo don't panic?
            Err(e) => panic!(
                "This is a bug :(\n{}",
                e.display(
                    ast_formatter.source(),
                    ast_formatter.pos(),
                    crate_source.path()
                )
            ),
        }
        Ok(ast_formatter.finish())
    })
}

fn parse_crate(
    crate_source: CrateSource,
) -> Result<(rustc_ast::ast::Crate, String), ErrorGuaranteed> {
    let crate_;
    let source;
    {
        // We don't share a ParseSess across crates because its SourceMap would hold every
        // crate's contents in memory after it is parsed and formatted
        let psess = build_parse_sess();
        // todo is this unwrap okay?
        let mut parser = rustc_parse::unwrap_or_emit_fatal(build_parser(&psess, crate_source));
        crate_ = parser.parse_crate_mod().map_err(|err| err.emit())?;
        if let Some(error) = psess.dcx().has_errors() {
            return Err(error);
        }
        let [file] = &psess.source_map().files()[..] else {
            panic!("the SourceMap should have exactly one file");
        };
        let source_ref = file.src.as_ref().expect("the SourceFile should have src");
        source = Arc::clone(source_ref);
        // after this block, `source` will be a single reference that we can unwrap
    };
    let source = Arc::into_inner(source)
        .expect("there should be no references to source");
    Ok((crate_, source))
}

fn build_parse_sess() -> ParseSess {
    let source_map = Arc::new(SourceMap::new(FilePathMapping::empty()));
    let dcx = build_diag_ctxt(source_map.clone());
    ParseSess::with_dcx(dcx, source_map)
}

fn build_diag_ctxt(source_map: Arc<SourceMap>) -> DiagCtxt {
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

fn build_parser<'a>(
    psess: &'a ParseSess,
    source: CrateSource,
) -> Result<Parser<'a>, Vec<Diag<'a>>> {
    match source {
        CrateSource::File(path) => rustc_parse::new_parser_from_file(psess, &path, None // todo provide span when the file is found from a mod
),
        CrateSource::Source(source) => rustc_parse::new_parser_from_source_str(
            psess,
            FileName::anon_source_code(&source),
            source.to_owned(),
        ),
    }
}
