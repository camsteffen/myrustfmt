use crate::CrateSource;
use crate::ast_module::AstModule;
use crate::module_extras::get_module_extras;
use crate::submodules::Submodule;
use rustc_errors::ColorConfig;
use rustc_errors::DiagCtxt;
use rustc_errors::ErrorGuaranteed;
use rustc_errors::PResult;
use rustc_errors::emitter::HumanEmitter;
use rustc_errors::emitter::stderr_destination;
use rustc_parse::parser::ExpTokenPair;
use rustc_parse::parser::Parser;
use rustc_session::parse::ParseSess;
use rustc_span::source_map::{FilePathMapping, SourceMap};
use rustc_span::symbol::Ident;
use rustc_span::{FileName, SourceFile};
use std::sync::Arc;

pub struct ParseModuleResult {
    pub module: AstModule,
    pub source_file: SourceFile,
    pub submodules: Vec<Submodule>,
}

pub fn parse_module(
    crate_source: CrateSource,
    relative: Option<Ident>,
) -> Result<ParseModuleResult, ErrorGuaranteed> {
    let module;
    let submodules;
    let source_file;
    // This block ensures we have a unique reference to the SourceFile at the end.
    {
        // Create a fresh SourceMap, ParseSess, etc. for every file to avoid unnecessarily
        // accumulating files in memory.
        let source_map = Arc::new(SourceMap::new(FilePathMapping::empty()));
        let dcx = build_diag_ctxt(Arc::clone(&source_map));
        let psess = ParseSess::with_dcx(dcx, Arc::clone(&source_map));

        let parser = module_parser(&psess, crate_source);
        let (attrs, items, spans) = parse_no_errors(parser, |parser| {
            parser.parse_mod(ExpTokenPair {
                tok: &rustc_ast::token::Eof,
                token_type: rustc_parse::parser::token_type::TokenType::Eof,
            })
        })?;

        let macro_args;
        (submodules, macro_args) = get_module_extras(&psess, &items, crate_source.path(), relative);

        module = AstModule {
            attrs,
            items,
            macro_args,
            spans,
        };

        source_file = match source_map.files().as_slice() {
            [file] => Arc::clone(file),
            _ => panic!("the SourceMap should have exactly one SourceFile"),
        };
    }

    let source_file =
        Arc::into_inner(source_file).expect("should have a unique reference to the SourceFile");

    Ok(ParseModuleResult {
        module,
        source_file,
        submodules,
    })
}

pub fn parse_no_errors<T>(
    mut parser: Parser,
    f: impl for<'p> FnOnce(&mut Parser<'p>) -> PResult<'p, T>,
) -> Result<T, ErrorGuaranteed> {
    match f(&mut parser) {
        Ok(value) => {
            if let Some(err) = parser.psess.dcx().has_errors() {
                parser.psess.dcx().reset_err_count();
                Err(err)
            } else {
                Ok(value)
            }
        }
        Err(diag) => {
            parser.psess.dcx().reset_err_count();
            Err(diag.emit())
        }
    }
}

fn build_diag_ctxt(source_map: Arc<SourceMap>) -> DiagCtxt {
    let translator = rustc_driver::default_translator();
    let emitter = Box::new(
        HumanEmitter::new(stderr_destination(ColorConfig::Auto), translator).sm(Some(source_map)),
    );
    DiagCtxt::new(emitter)
}

fn module_parser<'a>(psess: &'a ParseSess, source: CrateSource) -> Parser<'a> {
    let parser = match source {
        // todo provide span when the file is found from a mod
        CrateSource::File(path) => rustc_parse::new_parser_from_file(psess, path, None),
        CrateSource::Source(source) => rustc_parse::new_parser_from_source_str(
            psess,
            FileName::anon_source_code(source),
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
        rustc_span::create_session_globals_then(Edition::Edition2024, &[], None, || {
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
        rustc_span::create_session_globals_then(Edition::Edition2024, &[], None, || {
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
