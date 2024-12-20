#![feature(rustc_private)]

use std::ffi::OsStr;
use myrustfmt::format_file;
use std::{fs, io};
use std::path::Path;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn dogfood_test() -> io::Result<()> {
    dogfood_test_dir("./src".as_ref())?;
    // dogfood_test_file("./src/lib.rs");
    // dogfood_test_file("./src/ast_formatter.rs");
    // dogfood_test_file("./src/config.rs");
    // dogfood_test_file("./src/constraint_writer.rs");
    Ok(())
}

fn dogfood_test_dir(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            dogfood_test_dir(&entry.path())?;
        } else if entry.path().extension() == Some(OsStr::new("rs")) {
            dogfood_test_file(&entry.path());
            
        }
    }
    Ok(())
}

fn dogfood_test_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    println!("Testing file {}", path.display());
    let result = format_file(path);
    let original = fs::read_to_string(path).unwrap();
    assert_eq!(result, original)
}
