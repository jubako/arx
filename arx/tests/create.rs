mod utils;

use std::{io::Read, path::Path};
use utils::*;

#[test]
fn test_crate_non_existant_input() -> Result {
    temp_arx!(arx_file);
    let output = run!(
        output,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "non_existant_dir"
    );
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!(
        "[ERROR arx] Error : Input non_existant_dir path doesn't exist or cannot be accessed\n",
        stderr
    );
    assert!(!output.status.success());
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_non_existant_output_directory() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file, "non_existant_directory/test.arx");
    let output = run!(
        output,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap()
    );
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!(
        format!(
            "[ERROR arx] Error : Directory {} doesn't exist\n",
            arx_file.parent().unwrap().display()
        ),
        stderr
    );
    assert!(!output.status.success());
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_existant_output() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&arx_file)?;
        f.write_all(b"Some dummy content")?;
    }

    // Try to write without --force
    let output = run!(
        output,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap()
    );
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!(
        format!(
            "[ERROR arx] Error : File {} already exists. Use option --force to overwrite it.\n",
            arx_file.display()
        ),
        stderr
    );
    assert!(!output.status.success());
    assert_eq!(std::fs::read(&arx_file)?, b"Some dummy content");

    // Try to write without --force
    let output = run!(
        output,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap(),
        "--force"
    );
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!("", stderr);
    assert!(output.status.success());
    {
        let mut f = std::fs::File::open(&arx_file)?;
        let mut buf = [0; 10];
        f.read_exact(&mut buf)?;
        assert_eq!(&buf, b"jbkC\x00\x00\x00\x00\x00\x02");
    }
    Ok(())
}
