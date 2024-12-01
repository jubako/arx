mod utils;

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};
use utils::*;

pub struct TmpArx {
    _tmp: tempfile::TempDir,
    path: PathBuf,
}

impl TmpArx {
    pub(self) fn new(tmp_dir: tempfile::TempDir, path: PathBuf) -> Self {
        Self {
            _tmp: tmp_dir,
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub static BASE_ARX_FILE: LazyLock<TmpArx> = LazyLock::new(|| {
    let source_dir = SHARED_TEST_DIR.path();
    let tmp_arx_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_arx = tmp_arx_dir.path().join("test.arx");
    let output = cmd!(
        "arx",
        "create",
        "--outfile",
        &tmp_arx,
        "-C",
        source_dir.parent().unwrap(),
        "--strip-prefix",
        source_dir.file_name().unwrap(),
        source_dir.file_name().unwrap()
    )
    .output()
    .unwrap();
    println!("Out: {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    TmpArx::new(tmp_arx_dir, tmp_arx)
});

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_mount() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let mount_point = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))?;
    let arx = arx::Arx::new(arx_file)?;
    let arxfs = arx::ArxFs::new(arx)?;
    let _mount_handle = arxfs.spawn_mount("Test mounted arx".into(), mount_point.path())?;
    let output = run!(output, "diff", "-r", tmp_source_dir, mount_point.path());
    println!("Out: {}", String::from_utf8(output.stdout)?);
    println!("Err: {}", String::from_utf8(output.stderr)?);
    assert!(output.status.success());
    Ok(())
}

#[test]
fn test_extract() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))?;
    arx::extract(
        &arx_file,
        extract_dir.path(),
        Default::default(),
        true,
        false,
    )?;
    let output = run!(output, "diff", "-r", tmp_source_dir, extract_dir.path());
    println!("Out : {}", String::from_utf8(output.stdout)?);
    println!("Err: {}", String::from_utf8(output.stderr)?);
    assert!(output.status.success());
    Ok(())
}

#[test]
fn test_extract_filter() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;
    arx::extract(
        &arx_file,
        extract_dir.path(),
        ["sub_dir_a".into()].into(),
        true,
        true,
    )?;

    let mut source_sub_dir = tmp_source_dir.to_path_buf();
    source_sub_dir.push("sub_dir_a");
    let mut extract_sub_dir = extract_dir.path().to_path_buf();
    extract_sub_dir.push("sub_dir_a");

    println!(
        "Diff {} and {}",
        source_sub_dir.display(),
        extract_sub_dir.display()
    );
    let output = run!(output, "diff", "-r", &source_sub_dir, &extract_sub_dir);
    println!("Out : {}", String::from_utf8(output.stdout)?);
    println!("Err: {}", String::from_utf8(output.stderr)?);
    assert!(output.status.success());
    Ok(())
}

#[test]
fn test_extract_subdir() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    let output = run!(
        output,
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
    let output = run!(output, "diff", "-r", &source_sub_dir, extract_dir.path());
    println!("Out: {}", String::from_utf8(output.stdout)?);
    println!("Err: {}", String::from_utf8(output.stderr)?);
    assert!(output.status.success());
    Ok(())
}

#[test]
fn test_extract_subfile() -> Result {
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    let output = run!(
        output,
        "arx",
        "extract",
        &arx_file,
        "--root-dir",
        "sub_dir_a/file1.txt",
        "-C",
        extract_dir.path()
    );
    assert!(!output.status.success());
    Ok(())
}
