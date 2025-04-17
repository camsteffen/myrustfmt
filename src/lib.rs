#![feature(rustc_private)]
#![feature(let_chains)]
#![feature(precise_capturing_in_traits)]
#![feature(slice_take)]
#![feature(unqualified_local_imports)]
// Uncomment to let clippy babble (with some overrides made below)
// #![warn(clippy::pedantic)]
#![warn(
    unqualified_local_imports,
    clippy::inconsistent_struct_constructor,
    clippy::uninlined_format_args,
    clippy::unnecessary_semicolon,
)]
#![allow(
    clippy::bool_assert_comparison,
    clippy::from_iter_instead_of_collect,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
)]

// these crates are loaded from the sysroot, so they need extern crate.
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_lexer;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate thin_vec;
extern crate core;

mod ast_formatter;
mod ast_module;
mod ast_utils;
pub mod config;
mod constraint_writer;
mod constraints;
mod error;
mod error_emitter;
mod num;
mod parse;
mod rustfmt_config_defaults;
mod source_formatter;
mod submodules;
mod util;
mod whitespace;

use crate::ast_formatter::format_module;
use crate::config::Config;
use crate::parse::{ParseModuleResult, parse_module};
use crate::submodules::Submodule;
use rustc_span::ErrorGuaranteed;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use std::collections::VecDeque;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

#[derive(Debug)]
pub struct FormatModuleResult {
    pub error_count: u32,
    pub formatted: String,
}

impl FormatModuleResult {
    pub fn into_result(self) -> Result<String, Box<dyn Error>> {
        let Self {
            error_count,
            formatted,
        } = self;
        if error_count > 0 {
            return Err(
                format!("Some errors occurred. Formatted:\n{}", formatted)
                    .into(),
            );
        }
        Ok(formatted)
    }

    pub fn expect_no_errors(self) -> String {
        let Self {
            error_count,
            formatted,
        } = self;
        assert_eq!(error_count, 0, "Some errors occurred. Formatted:\n{}", formatted);
        formatted
    }
}

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

struct OnFormatModule {
    is_check: bool,
    is_verbose: bool,
    has_errors: bool,
}

impl OnFormatModule {
    fn on_format_module(
        &mut self,
        path: &Path,
        result: FormatModuleResult,
        source: &str,
    ) -> ControlFlow<()> {
        let FormatModuleResult {
            error_count,
            formatted,
        } = result;
        if error_count > 0 {
            self.has_errors = true;
        }
        if self.is_check {
            return self.check_file(path, source, &formatted);
        }
        if formatted == source {
            if self.is_verbose {
                eprintln!("Already formatted: {}", path.display());
            }
        } else {
            fs::write(path, formatted).unwrap();
            if self.is_verbose {
                eprintln!("Formatted: {}", path.display());
            }
        }
        ControlFlow::Continue(())
    }

    fn check_file(&self, path: &Path, contents: &str, formatted: &str) -> ControlFlow<()> {
        if contents == formatted {
            if self.is_verbose {
                eprintln!("Ok: {}", path.display());
            }
            return ControlFlow::Continue(());
        }
        eprintln!("Mismatch for {}", path.display());
        let mut child = Command::new("diff")
            .arg("--color")
            .arg(path)
            .arg("-")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(formatted.as_bytes()).unwrap();
        }
        child.wait().unwrap();
        // todo continue?
        ControlFlow::Break(())
    }
}

pub fn format_module_file_roots(
    paths: Vec<String>,
    config: Config,
    is_check: bool,
    is_verbose: bool,
) -> Result<(), ()> {
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let config = Rc::new(config);
        let mut queue = VecDeque::<(PathBuf, Option<Ident>)>::from_iter(
            paths.into_iter().map(|path| (path.into(), None)),
        );
        let mut on_format_module = OnFormatModule {
            is_check,
            is_verbose,
            has_errors: false,
        };
        while let Some((path, relative)) = queue.pop_front() {
            let submodules = format_module_file(&path, relative, &config, &mut on_format_module)?;
            queue.extend(
                submodules
                    .into_iter()
                    .map(|submod| (submod.path, submod.relative)),
            );
        }
        if on_format_module.has_errors {
            return Err(());
        }
        Ok(())
    })
}

fn format_module_file(
    path: &Path,
    relative: Option<Ident>,
    config: &Config,
    on_format_module: &mut OnFormatModule,
) -> Result<Vec<Submodule>, ()> {
    let result = parse_module(CrateSource::File(path), relative)
        .map_err(|ErrorGuaranteed { .. }| ())?;
    let ParseModuleResult {
        module,
        source,
        submodules,
    } = result;
    let source = Rc::new(source);
    let result = format_module(
        &module,
        Rc::clone(&source),
        Some(path.to_path_buf()),
        config,
    );
    match on_format_module.on_format_module(path, result, &source) {
        ControlFlow::Continue(()) => Ok(submodules),
        ControlFlow::Break(()) => Err(()),
    }
}

pub fn format_str(source: &str, config: Config) -> Result<FormatModuleResult, ErrorGuaranteed> {
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let ParseModuleResult {
            module,
            source,
            submodules: _,
        } = parse_module(CrateSource::Source(source), None)?;
        Ok(format_module(&module, Rc::new(source), None, &config))
    })
}
