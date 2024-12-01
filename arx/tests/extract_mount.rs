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
    cmd!(
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
    .check();
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
    assert!(tree_equal(tmp_source_dir, mount_point)?);
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
    assert!(tree_equal(tmp_source_dir, extract_dir)?);
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

    let source_sub_dir = join!(tmp_source_dir / "sub_dir_a");
    let extract_sub_dir = join!((extract_dir.path()) / "sub_dir_a");

    assert!(tree_equal(source_sub_dir, extract_sub_dir)?);
    Ok(())
}

#[test]
fn test_extract_subdir() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "--root-dir",
        "sub_dir_a",
        "-C",
        extract_dir.path()
    )
    .check_output(b"", b"");

    let source_sub_dir = join!(tmp_source_dir / "sub_dir_a");

    assert!(tree_equal(source_sub_dir, extract_dir)?);
    Ok(())
}

#[test]
fn test_extract_subfile() -> Result {
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "--root-dir",
        "sub_dir_a/file1.txt",
        "-C",
        extract_dir.path()
    )
    .check_fail(
        b"",
        b"[ERROR arx] Error : sub_dir_a/file1.txt must be a directory\n",
    );
    Ok(())
}
