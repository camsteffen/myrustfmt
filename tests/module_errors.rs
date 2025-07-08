mod util;

use crate::util::SimpleOutput;
use std::fs;
use std::process::Command;

#[test]
fn test() {
    let output = Command::new(env!("CARGO_BIN_EXE_myrustfmt"))
        .current_dir("tests/module_errors")
        .arg("test.rs")
        .output()
        .unwrap();
    assert_eq!(
        SimpleOutput::expect(output),
        SimpleOutput {
            stderr: fs::read_to_string("tests/module_errors/stderr.txt").unwrap(),
            stdout: String::new(),
            code: 1,
        },
    );
}
