#![feature(rustc_private)]

use myrustfmt::config::Config;
use myrustfmt::format_file;
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let args = Vec::from_iter(env::args());
    if args.len() < 2 {
        eprintln!("Provide a path");
        return ExitCode::FAILURE;
    }
    if args.len() > 2 {
        eprintln!("Too many arguments");
        return ExitCode::FAILURE;
    }
    let path = &args[1];
    let formatted = format_file(path, Config::default()).unwrap();
    fs::write(path, formatted).unwrap();
    ExitCode::SUCCESS
}
