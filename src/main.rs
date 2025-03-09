#![feature(rustc_private)]

extern crate rustc_span;

use getopts::Options;
use myrustfmt::config::Config;
use myrustfmt::format_module_file_roots;
use std::env;
use std::process::ExitCode;

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
    // todo dedupe files and their submodules (two files can have a shared submodule, like in tests/)
    let paths = options_matches.free;
    match format_module_file_roots(paths, Config::default(), is_check, is_verbose) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
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
    opts.optflag("v", "verbose", "Print verbose output");
    opts
}
