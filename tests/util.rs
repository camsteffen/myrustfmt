use std::process::Output;

#[derive(Debug, PartialEq)]
pub struct SimpleOutput {
    pub code: i32,
    pub stderr: String,
    pub stdout: String,
}

impl SimpleOutput {
    pub fn expect(output: Output) -> Self {
        let Output {
            stderr,
            stdout,
            status,
        } = output;
        SimpleOutput {
            code: status.code().expect("output does not have a status code"),
            stderr: String::from_utf8(stderr).expect("stderr is not valid UTF-8"),
            stdout: String::from_utf8(stdout).expect("stdout is not valid UTF-8"),
        }
    }
}
