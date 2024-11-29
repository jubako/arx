mod utils;

use std::{io::Read, path::Path};
use utils::*;

#[test]
fn test_crate_non_existant_input() {
    temp_arx!(arx_file);
    let output = cmd!("arx", "create", "--outfile", &arx_file, "non_existant_dir");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!(
        "[ERROR arx] Error : Input non_existant_dir path doesn't exist or cannot be accessed\n",
        stderr
    );
    assert!(!output.status.success());
    assert!(!arx_file.exists());
}

#[test]
fn test_crate_non_existant_output_directory() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file, "non_existant_directory/test.arx");
    let output = cmd!(
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
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
}

#[test]
fn test_crate_existant_output() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&arx_file).unwrap();
        f.write_all(b"Some dummy content").unwrap();
    }

    // Try to write without --force
    let output = cmd!(
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
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
    assert_eq!(std::fs::read(&arx_file).unwrap(), b"Some dummy content");

    // Try to write without --force
    let output = cmd!(
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    println!("Out : {}", stdout);
    println!("Err : {}", stderr);
    assert_eq!("", stdout);
    assert_eq!("", stderr);
    assert!(output.status.success());
    {
        let mut f = std::fs::File::open(&arx_file).unwrap();
        let mut buf = [0; 10];
        f.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"jbkC\x00\x00\x00\x00\x00\x02");
    }
}

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_create_and_mount() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    let output = cmd!(
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
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let mount_point = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let arx = arx::Arx::new(arx_file).unwrap();
    let arxfs = arx::ArxFs::new(arx).unwrap();
    let _mount_handle = arxfs
        .spawn_mount("Test mounted arx".into(), mount_point.path())
        .unwrap();
    let output = cmd!("diff", "-r", tmp_source_dir, mount_point.path());
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
}

#[test]
fn test_create_and_extract() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    let output = cmd!(
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
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let extract_dir = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    arx::extract(
        &arx_file,
        extract_dir.path(),
        Default::default(),
        true,
        false,
    )
    .unwrap();
    let output = cmd!("diff", "-r", tmp_source_dir, extract_dir.path());
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
}

#[test]
fn test_create_and_extract_filter() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    let output = cmd!(
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
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let extract_dir =
        tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR")).unwrap();
    arx::extract(
        &arx_file,
        extract_dir.path(),
        ["sub_dir_a".into()].into(),
        true,
        true,
    )
    .unwrap();

    let mut source_sub_dir = tmp_source_dir.to_path_buf();
    source_sub_dir.push("sub_dir_a");
    let mut extract_sub_dir = extract_dir.path().to_path_buf();
    extract_sub_dir.push("sub_dir_a");

    println!(
        "Diff {} and {}",
        source_sub_dir.display(),
        extract_sub_dir.display()
    );
    let output = cmd!("diff", "-r", &source_sub_dir, &extract_sub_dir);
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
}

#[test]
fn test_create_and_extract_subdir() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    let output = cmd!(
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
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let extract_dir =
        tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR")).unwrap();

    let output = cmd!(
        "arx",
        "extract",
        &arx_file,
        "--root-dir",
        "sub_dir_a",
        "-C",
        extract_dir.path()
    );
    assert!(output.status.success());

    let mut source_sub_dir = tmp_source_dir.to_path_buf();
    source_sub_dir.push("sub_dir_a");

    println!(
        "Diff {} and {}",
        source_sub_dir.display(),
        extract_dir.path().display()
    );
    let output = cmd!("diff", "-r", &source_sub_dir, extract_dir.path());
    println!("Out: {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
}

#[test]
fn test_create_and_extract_subfile() {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_arx!(arx_file);
    let output = cmd!(
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
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let extract_dir =
        tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR")).unwrap();

    let output = cmd!(
        "arx",
        "extract",
        &arx_file,
        "--root-dir",
        "sub_dir_a/file1.txt",
        "-C",
        extract_dir.path()
    );
    assert!(!output.status.success());
}
