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

pub mod formatter;
pub mod writer;

use rustc_data_structures::sync::Lrc;
use rustc_errors::emitter::{stderr_destination, HumanEmitter};
use rustc_errors::{ColorConfig, DiagCtxt};
use rustc_lexer::TokenKind;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use rustc_span::{
    source_map::{FilePathMapping, SourceMap},
    BytePos, FileName, Pos, Span,
};

use formatter::Formatter;

pub fn format_str(source: &str, max_width: usize) -> String {
    let crate_ = parse_ast(String::from(source));
    let mut parse_tree = Formatter::new(source, max_width);
    match parse_tree.crate_(&crate_) {
        Ok(()) => {}
        Err(e) => todo!("failed to format: {e:?}"),
    }
    parse_tree.finish()
}

fn parse_ast(string: String) -> rustc_ast::ast::Crate {
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
        parser.parse_crate_mod().unwrap_or_else(|err| {
            err.emit();
            panic!("ur done");
        })
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
