mod utils;

use rustest::{test, *};

use format_bytes::format_bytes;
use std::{io::Read, path::Path};
use utils::*;

#[test]
fn test_crate_non_existant_input() -> Result {
    temp_arx!(arx_file);
    cmd!("arx", "create", "--outfile", &arx_file, "non_existant_dir").check_fail(
        b"",
        b"Error : Input non_existant_dir path doesn't exist or cannot be accessed\n",
    );
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_non_existant_output_directory(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file, "non_existant_directory/test.arx");
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        source_dir.parent().unwrap(),
        "--strip-prefix",
        source_dir.file_name().unwrap(),
        source_dir.file_name().unwrap()
    )
    .check_fail(
        b"",
        &format_bytes!(
            b"Error : Directory {} doesn't exist\n",
            arx_file.parent().unwrap().as_os_str().as_encoded_bytes()
        ),
    );
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_existant_output(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&arx_file)?;
        f.write_all(b"Some dummy content")?;
    }

    // Try to write without --force
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        source_dir.parent().unwrap(),
        "--strip-prefix",
        source_dir.file_name().unwrap(),
        source_dir.file_name().unwrap()
    )
    .check_fail(
        b"",
        &format_bytes!(
            b"Error : File {} already exists. Use option --force to overwrite it.\n",
            arx_file.as_os_str().as_encoded_bytes()
        ),
    );
    assert_eq!(std::fs::read(&arx_file)?, b"Some dummy content");

    // Try to write without --force
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        source_dir.parent().unwrap(),
        "--strip-prefix",
        source_dir.file_name().unwrap(),
        source_dir.file_name().unwrap(),
        "--force"
    )
    .check_output(Some(b""), Some(b""));
    {
        let mut f = std::fs::File::open(&arx_file)?;
        let mut buf = [0; 10];
        f.read_exact(&mut buf)?;
        assert_eq!(&buf, b"jbkC\x00\x00\x00\x00\x00\x02");
    }
    Ok(())
}

#[rustest::main]
fn main() {}
