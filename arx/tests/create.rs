mod utils;

use rustest::{test, *};

use std::{io::Read, path::Path};
use utils::*;

#[test]
fn test_crate_non_existant_input() -> Result {
    temp_arx!(arx_file);
    cmd!("arx", "create", "--outfile", &arx_file, "non_existant_dir").check_fail(
        "",
        "Error : Input non_existant_dir path doesn't exist or cannot be accessed\n",
    );
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_non_existant_output_directory(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file, "non_existant_directory/test.arx");
    cmd!("arx", "create", "--outfile", &arx_file, source_dir).check_fail(
        "",
        &regex::escape(&format!(
            "Error : Directory {} doesn't exist\n",
            arx_file.parent().unwrap().to_str().unwrap()
        )),
    );
    assert!(!arx_file.exists());
    Ok(())
}

#[test]
fn test_crate_dir_as_root(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    let status = run!(
        status,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        join!(source_dir / "sub_dir_a"),
        "--dir-as-root"
    );
    assert!(status.success());
    let arx_content = run!(output, "arx", "list", &arx_file);
    let arx_content = String::from_utf8_lossy(&arx_content.stdout);
    let arx_content = arx_content.lines().collect::<Vec<_>>();

    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a"),
        join!(source_dir / "sub_dir_a"),
    )?);
    Ok(())
}

#[test]
fn test_crate_trim(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    run!(
        status,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        join!(source_dir / "sub_dir_a")
    );
    let arx_content = run!(output, "arx", "list", &arx_file);
    let arx_content = String::from_utf8_lossy(&arx_content.stdout);
    let arx_content = arx_content.lines().collect::<Vec<_>>();

    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a"),
        source_dir
    )?);
    Ok(())
}

#[test]
fn test_crate_keep(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    let current_dir = source_dir.parent().unwrap();
    let dir_to_add = join!(source_dir / "sub_dir_a");
    let relative_path = dir_to_add.strip_prefix(current_dir).unwrap();
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        &relative_path,
        "-k"
    )
    .current_dir(current_dir)
    .status()?;
    let arx_content = run!(output, "arx", "list", &arx_file);
    let arx_content = String::from_utf8_lossy(&arx_content.stdout);
    let arx_content = arx_content.lines().collect::<Vec<_>>();

    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a"),
        source_dir.parent().unwrap()
    )?);
    Ok(())
}

#[test]
fn test_crate_several_input(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    let dir_a_to_add = join!(source_dir / "sub_dir_a");
    let dir_b_to_add = join!(source_dir / "sub_dir_b");
    let file_c_to_add = join!(source_dir / "sub_dir_c" / "existing_file");
    run!(
        status,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        &dir_a_to_add,
        &dir_b_to_add,
        &file_c_to_add
    );
    let arx_content = run!(output, "arx", "list", &arx_file);
    let arx_content = String::from_utf8_lossy(&arx_content.stdout);
    let arx_content = arx_content.lines().collect::<Vec<_>>();

    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a"),
        source_dir
    )?);
    Ok(())
}

#[test]
fn test_crate_several_input_root_as_dir(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    let dir_a_to_add = join!(source_dir / "sub_dir_a");
    let dir_b_to_add = join!(source_dir / "sub_dir_a_bis");
    run!(
        status,
        "arx",
        "create",
        "--outfile",
        &arx_file,
        &dir_a_to_add,
        &dir_b_to_add,
        "--dir-as-root"
    );
    let arx_content = run!(output, "arx", "list", &arx_file);
    let arx_content = String::from_utf8_lossy(&arx_content.stdout);
    let arx_content = arx_content.lines().collect::<Vec<_>>();

    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a"),
        join!(source_dir / "sub_dir_a"),
    )?);
    assert!(list_diff(
        &arx_content,
        join!(source_dir / "sub_dir_a_bis"),
        join!(source_dir / "sub_dir_a_bis"),
    )?);
    Ok(())
}

#[test]
fn test_crate_several_input_root_as_dir_duplicate(source_dir: SharedTestDir) -> Result {
    let source_dir = source_dir.path();
    temp_arx!(arx_file);
    let dir_a_to_add = join!(source_dir / "sub_dir_a");
    let dir_b_to_add = join!(source_dir / "sub_dir_b");
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        &dir_a_to_add,
        &dir_b_to_add,
        "--dir-as-root"
    )
    .check_fail(
        "",
        "Error : Incoherent structure : Adding file0.bin, cannot add a file when one already exists\n",
    );
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
    cmd!("arx", "create", "--outfile", &arx_file, source_dir).check_fail(
        "",
        &regex::escape(&format!(
            "Error : File {} already exists. Use option --force to overwrite it.\n",
            arx_file.to_str().unwrap()
        )),
    );
    assert_eq!(std::fs::read(&arx_file)?, b"Some dummy content");

    // Try to write without --force
    cmd!(
        "arx",
        "create",
        "--outfile",
        &arx_file,
        source_dir,
        "--force"
    )
    .check_output(Some(""), Some(""));
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
