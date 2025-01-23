use crate::CrateSource;
use rustc_errors::ColorConfig;
use rustc_errors::DiagCtxt;
use rustc_errors::ErrorGuaranteed;
use rustc_errors::emitter::HumanEmitter;
use rustc_errors::emitter::stderr_destination;
use rustc_parse::parser::Parser;
use rustc_session::parse::ParseSess;
use rustc_span::FileName;
use rustc_span::source_map::FilePathMapping;
use rustc_span::source_map::SourceMap;
use std::sync::Arc;

pub fn parse_crate(
    crate_source: CrateSource,
) -> Result<(rustc_ast::ast::Crate, String), ErrorGuaranteed> {
    // We don't share a ParseSess across crates because its SourceMap would hold every
    // crate's contents in memory with no way to clear it
    let psess = build_parse_sess();

    let mut parser = build_parser(&psess, crate_source);

    let source = {
        // the SourceMap entry is inserted when the parser is created above
        let [file] = &psess.source_map().files()[..] else {
            panic!("the SourceMap should have exactly one file");
        };
        Arc::clone(file.src.as_ref().expect("the SourceFile should have src"))
    };

    let crate_ = parser.parse_crate_mod().map_err(|err| err.emit())?;
    if let Some(error) = psess.dcx().has_errors() {
        return Err(error);
    }

    // drop ParseSess so that `source` is a single reference, then unwrap it
    drop(psess);
    let source = Arc::into_inner(source).expect("there should be no references to source");

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

fn build_parser<'a>(psess: &'a ParseSess, source: CrateSource) -> Parser<'a> {
    let parser = match source {
        // todo provide span when the file is found from a mod
        CrateSource::File(path) => rustc_parse::new_parser_from_file(psess, &path, None),
        CrateSource::Source(source) => rustc_parse::new_parser_from_source_str(
            psess,
            FileName::anon_source_code(&source),
            source.to_owned(),
        ),
    };
    // todo is this unwrap okay?
    rustc_parse::unwrap_or_emit_fatal(parser)
}
