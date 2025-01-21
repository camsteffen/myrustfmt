#![feature(rustc_private)]

use getopts::Options;
use myrustfmt::config::Config;
use myrustfmt::{CrateFormatError, format_file};
use std::io::Write;
use std::process::{Command, ExitCode, Stdio};
use std::{env, fs};

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
    let paths = options_matches.free;
    for path in paths {
        let format_result = format_file(&path, Config::default());
        let (contents, formatted) = match format_result {
            Ok(formatted) => formatted,
            Err(CrateFormatError::ErrorEmitted) => return ExitCode::FAILURE,
            Err(CrateFormatError::WidthLimitExceeded { line }) => {
                eprintln!("Width limit exceeded at {path}:{line}");
                return ExitCode::FAILURE;
            }
        };
        if is_check {
            if !check_file(&path, &contents, &formatted) {
                return ExitCode::FAILURE;
            }
        } else {
            fs::write(path, formatted).unwrap();
        }
    }
    ExitCode::SUCCESS
}

fn check_file(path: &str, contents: &str, formatted: &str) -> bool {
    if contents == formatted {
        return true;
    }
    eprintln!("Mismatch for {path}");
    let mut child = Command::new("diff")
        .arg("--color")
        .arg(&path)
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

fn build_options() -> Options {
    let mut opts = Options::new();

    opts.optflag(
        "",
        "check",
        "Run in 'check' mode. Exits with 0 if input is formatted correctly. Exits \
         with 1 and prints a diff if formatting is required.",
    );
    // let is_nightly = is_nightly();
    // let emit_opts = if is_nightly {
    //     "[files|stdout|coverage|checkstyle|json]"
    // } else {
    //     "[files|stdout]"
    // };
    // opts.optopt("", "emit", "What data to emit and how", emit_opts);
    // opts.optflag("", "backup", "Backup any modified files.");
    // opts.optopt(
    //     "",
    //     "config-path",
    //     "Recursively searches the given path for the rustfmt.toml config file. If not \
    //      found reverts to the input file path",
    //     "[Path for the configuration file]",
    // );
    opts.optopt(
        "",
        "edition",
        "Rust edition to use",
        "[2015|2018|2021|2024]",
    );
    // opts.optopt(
    //     "",
    //     "color",
    //     "Use colored output (if supported)",
    //     "[always|never|auto]",
    // );
    // opts.optopt(
    //     "",
    //     "print-config",
    //     "Dumps a default or minimal config to PATH. A minimal config is the \
    //      subset of the current config file used for formatting the current program. \
    //      `current` writes to stdout current config as if formatting the file at PATH.",
    //     "[default|minimal|current] PATH",
    // );
    // opts.optflag(
    //     "l",
    //     "files-with-diff",
    //     "Prints the names of mismatched files that were formatted. Prints the names of \
    //      files that would be formatted when used with `--check` mode. ",
    // );
    // opts.optmulti(
    //     "",
    //     "config",
    //     "Set options from command line. These settings take priority over .rustfmt.toml",
    //     "[key1=val1,key2=val2...]",
    // );

    // opts.optflag("v", "verbose", "Print verbose output");
    // opts.optflag("q", "quiet", "Print less output");
    // opts.optflag("V", "version", "Show version information");
    // let help_topics = if is_nightly {
    //     "`config` or `file-lines`"
    // } else {
    //     "`config`"
    // };
    // let mut help_topic_msg = "Show this message or help about a specific topic: ".to_owned();
    // help_topic_msg.push_str(help_topics);

    // opts.optflagopt("h", "help", &help_topic_msg, "=TOPIC");

    opts
}
