use crate::CrateSource;
use crate::ast_module::AstModule;
use crate::submodules::{Submodule, get_submodules};
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
use rustc_span::symbol::Ident;
use rustc_parse::parser::ExpTokenPair;
use std::sync::Arc;

pub struct ParseModuleResult {
    pub module: AstModule,
    pub source: String,
    pub submodules: Vec<Submodule>,
}

pub fn parse_module(
    crate_source: CrateSource,
    relative: Option<Ident>,
) -> Result<ParseModuleResult, ErrorGuaranteed> {
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

    let (attrs, items, spans) = parser
        .parse_mod(ExpTokenPair {
            tok: &rustc_ast::token::Eof,
            token_type: rustc_parse::parser::token_type::TokenType::Eof
        })
        .map_err(|err| err.emit())?;
    let module = AstModule {
        attrs,
        items,
        spans,
    };
    if let Some(error) = psess.dcx().has_errors() {
        return Err(error);
    }

    let submodules = match crate_source {
        CrateSource::File(path) => get_submodules(&psess, &module, path, relative),
        CrateSource::Source(_) => Vec::new(),
    };

    // drop ParseSess so that `source` is a single reference, then unwrap it
    drop(psess);
    let source = Arc::into_inner(source).expect("there should be no references to source");

    Ok(ParseModuleResult {
        module,
        source,
        submodules,
    })
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

#[cfg(test)]
mod tests {
    use crate::CrateSource;
    use crate::parse::parse_module;
    use rustc_span::Symbol;
    use rustc_span::edition::Edition;
    use rustc_span::symbol::Ident;
    use std::path::Path;

    #[test]
    fn test_submodules_non_relative() {
        rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
            let module = parse_module(
                CrateSource::File(Path::new("tests/submodules_tests/non_relative/main.rs")),
                None,
            )
            .unwrap();
            let expected = &[
                (
                    "tests/submodules_tests/non_relative/inline/file.rs",
                    Some("file"),
                ),
                (
                    "tests/submodules_tests/non_relative/inline/folder/mod.rs",
                    None,
                ),
                ("tests/submodules_tests/non_relative/file.rs", Some("file")),
                ("tests/submodules_tests/non_relative/folder/mod.rs", None),
                (
                    "tests/submodules_tests/non_relative/path_attr/test.rs",
                    None,
                ),
                (
                    "tests/submodules_tests/non_relative/inline_path_value/file.rs",
                    Some("file"),
                ),
                (
                    "tests/submodules_tests/non_relative/inline_path_value/folder/mod.rs",
                    None,
                ),
                (
                    "tests/submodules_tests/non_relative/inline_path_value/path_attr_inside_value.rs",
                    None,
                ),
            ][..];
            let actual = Vec::from_iter(module.submodules.iter().map(|submodule| {
                (
                    submodule.path.to_str().unwrap(),
                    submodule.relative.as_ref().map(|i| i.as_str()),
                )
            }));

            assert_eq!(expected, &actual);
        })
    }

    #[test]
    fn test_submodules_relative() {
        rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
            let module = parse_module(
                CrateSource::File(Path::new("tests/submodules_tests/relative/main.rs")),
                Some(Ident::with_dummy_span(Symbol::intern("main"))),
            )
            .unwrap();
            let expected = &[
                (
                    "tests/submodules_tests/relative/main/inline/file.rs",
                    Some("file"),
                ),
                (
                    "tests/submodules_tests/relative/main/inline/folder/mod.rs",
                    None,
                ),
                ("tests/submodules_tests/relative/main/file.rs", Some("file")),
                ("tests/submodules_tests/relative/main/folder/mod.rs", None),
                ("tests/submodules_tests/relative/path_attr/test.rs", None),
                (
                    "tests/submodules_tests/relative/inline_path_value/file.rs",
                    Some("file"),
                ),
                (
                    "tests/submodules_tests/relative/inline_path_value/folder/mod.rs",
                    None,
                ),
                (
                    "tests/submodules_tests/relative/inline_path_value/path_attr_inside_value.rs",
                    None,
                ),
            ][..];
            let actual = Vec::from_iter(module.submodules.iter().map(|submodule| {
                (
                    submodule.path.to_str().unwrap(),
                    submodule.relative.as_ref().map(|i| i.as_str()),
                )
            }));

            assert_eq!(expected, &actual);
        })
    }
}
