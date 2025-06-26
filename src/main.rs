#![feature(rustc_private)]

extern crate rustc_span;

use getopts::Options;
use myrustfmt::config::Config;
use myrustfmt::{FormatModuleResult, format_module_file_roots, format_str};
use rustc_span::ErrorGuaranteed;
use std::io::{Write, stdin, stdout};
use std::process::ExitCode;
use std::{env, io};

fn main() -> ExitCode {
    let args = Vec::from_iter(env::args());
    if args.len() < 2 {
        eprintln!("Provide at least one path");
        return ExitCode::FAILURE;
    }
    let options = build_options();
    let options_matches = options.parse(&args[1..]).unwrap();
    if options_matches.opt_present("edition") {
        // todo wat
        eprintln!("WARNING: Ignoring --edition option");
    }
    let is_check = options_matches.opt_present("check");
    let is_verbose = options_matches.opt_present("verbose");
    let mut config = Config::default();
    if let Some(max_width) = options_matches.opt_str("max-width") {
        config.max_width = match max_width.parse() {
            Ok(val) => val,
            Err(_) => {
                eprintln!("Invalid max-width value");
                return ExitCode::FAILURE;
            }
        };
    }
    // todo dedupe files and their submodules (two files can have a shared submodule, like in tests/)
    let paths = options_matches.free;
    if let [path] = &paths[..]
        && path == "-"
    {
        return do_stdin(config);
    }
    match format_module_file_roots(paths, config, is_check, is_verbose) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

fn do_stdin(config: Config) -> ExitCode {
    let input = io::read_to_string(stdin()).expect("failed to read stdin");
    match format_str(&input, config) {
        Ok(
            FormatModuleResult {
                error_count,
                formatted,
            },
        ) => {
            stdout()
                .write_all(formatted.as_bytes())
                .expect("failed to write to stdout");
            // todo consolidate exit code logic with non stdin mode
            if error_count == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(ErrorGuaranteed { .. }) => ExitCode::FAILURE,
    }
}

fn build_options() -> Options {
    let mut opts = Options::new();
    opts.optflag(
        "",
        "check",
        "Run in 'check' mode. Exits with 0 if input is formatted correctly. Exits \
         with 1 and prints a diff if formatting is required.",
    );
    opts.optopt(
        "",
        "edition",
        "Rust edition to use",
        "[2015|2018|2021|2024]",
    );
    opts.optopt("", "max-width", "Maximum width of each line", "WIDTH");
    opts.optflag("v", "verbose", "Print verbose output");
    opts
}
