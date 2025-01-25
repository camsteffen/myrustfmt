#![feature(rustc_private)]

// these crates are loaded from the sysroot, so they need extern crate.
extern crate rustc_ast;
// extern crate rustc_ast_pretty;
// extern crate rustc_builtin_macros;
// extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_lexer;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate thin_vec;

pub mod ast_formatter;
mod ast_utils;
pub mod config;
pub mod constraint_writer;
mod constraints;
mod error;
mod error_emitter;
mod submodules;
mod parse;
mod rustfmt_config_defaults;
pub mod source_formatter;
mod source_reader;
mod util;
mod ast_module;

use crate::ast_formatter::{AstFormatter, FormatModuleResult};
use crate::config::Config;
use crate::submodules::Submodule;
use crate::parse::{ParseModuleResult, parse_module};
use rustc_span::ErrorGuaranteed;
use rustc_span::edition::Edition;
use rustc_span::symbol::Ident;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;

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

pub fn format_file(root_path: &Path, config: Config, is_check: bool) -> bool {
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        do_file(root_path, None, Rc::new(config), |path, result, source| {
            let FormatModuleResult {
                formatted,
                exceeded_max_width,
            } = result;
            if exceeded_max_width {
                return false;
            }
            if is_check {
                check_file(path, source, &formatted)
            } else if formatted != source {
                fs::write(path, formatted).unwrap();
                true
            } else {
                true
            }
        })
    })
}

// todo bool really?
fn do_file(
    path: &Path,
    relative: Option<Ident>,
    config: Rc<Config>,
    // todo rename
    reacter: impl Fn(&Path, FormatModuleResult, &str) -> bool + Copy,
) -> bool {
    match do_one(path, relative, Rc::clone(&config), reacter) {
        Err(()) => false,
        Ok(submodules) => {
            submodules.into_iter().all(|submodule| {
                do_file(
                    &submodule.path,
                    submodule.relative,
                    Rc::clone(&config),
                    reacter,
                )
            })
        },
    }
}

fn do_one(
    path: &Path,
    relative: Option<Ident>,
    config: Rc<Config>,
    reacter: impl Fn(&Path, FormatModuleResult, &str) -> bool + Copy,
) -> Result<Vec<Submodule>, ()> {
    let result = parse_module(CrateSource::File(path), relative);
    let result = match result {
        Ok(result) => result,
        Err(ErrorGuaranteed { .. }) => return Err(()),
    };
    let ParseModuleResult {
        module,
        source,
        submodules,
    } = result;
    let source = Rc::new(source);
    let ast_formatter = AstFormatter::new(
        Rc::clone(&source),
        Some(path.to_path_buf()),
        config,
    );
    let result = ast_formatter.module(&module);
    if !reacter(path, result, &source) {
        return Err(());
    }
    Ok(submodules)
}

pub fn format_str_defaults(source: &str) -> Result<FormatModuleResult, ErrorGuaranteed> {
    format_str_config(source, Config::default())
}

pub fn format_str(source: &str, max_width: u32) -> Result<FormatModuleResult, ErrorGuaranteed> {
    format_str_config(source, Config::default().max_width(max_width))
}

pub fn format_str_config(
    source: &str,
    config: Config,
) -> Result<FormatModuleResult, ErrorGuaranteed> {
    rustc_span::create_session_globals_then(Edition::Edition2024, None, || {
        let ParseModuleResult {
            module,
            source,
            submodules: _,
        } = parse_module(CrateSource::Source(source), None)?;
        let ast_formatter = AstFormatter::new(Rc::new(source), None, Rc::new(config));
        Ok(ast_formatter.module(&module))
    })
}

fn check_file(path: &Path, contents: &str, formatted: &str) -> bool {
    if contents == formatted {
        return true;
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
    false
}
