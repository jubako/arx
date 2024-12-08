mod utils;

use format_bytes::format_bytes;
use std::{
    ffi::OsStr,
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
    .check_output(Some(b""), Some(b""));
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
    assert!(tree_diff(mount_point, tmp_source_dir, SimpleDiffer::new())?);
    Ok(())
}

#[test]
fn test_extract() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))?;
    arx::extract(
        arx_file,
        extract_dir.path(),
        Default::default(),
        true,
        false,
        arx::Overwrite::Error,
    )?;
    assert!(tree_diff(extract_dir, tmp_source_dir, SimpleDiffer::new())?);
    Ok(())
}

#[test]
fn test_extract_same_dir() -> Result {
    // This test that everything go "fine" when extracting an archive in the source directory.
    // But here we don't want to take the risk to polute our source directory shared with other tests.
    // So we extract twice the same archive in the same place.

    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;
    cmd!("arx", "extract", &arx_file, "-C", extract_dir.path()).check_output(Some(b""), Some(b""));
    cmd!("arx", "extract", &arx_file, "-C", extract_dir.path()).check_output(Some(b""), None);
    Ok(())
}

#[test]
fn test_extract_filter() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;
    arx::extract(
        arx_file,
        extract_dir.path(),
        ["sub_dir_a".into()].into(),
        true,
        true,
        arx::Overwrite::Error,
    )?;

    let source_sub_dir = join!(tmp_source_dir / "sub_dir_a");
    let extract_sub_dir = join!(extract_dir / "sub_dir_a");

    assert!(tree_diff(
        extract_sub_dir,
        source_sub_dir,
        SimpleDiffer::new()
    )?);
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
    .check_output(Some(b""), Some(b""));

    let source_sub_dir = join!(tmp_source_dir / "sub_dir_a");

    assert!(tree_diff(extract_dir, source_sub_dir, SimpleDiffer::new())?);
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
    .check_fail(b"", b"Error : sub_dir_a/file1.txt must be a directory\n");
    Ok(())
}

#[test]
fn test_extract_existing_content_skip() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });
    let file_content = std::fs::read(join!(extract_dir / "sub_dir_a" / "existing_file"))?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=skip"
    )
    .check_output(Some(b""), Some(b""));
    assert!(tree_diff(
        extract_dir,
        tmp_source_dir,
        ExceptionDiffer::from([
            (
                join!("sub_dir_a" / "existing_file"),
                ExistingExpected::Content(file_content)
            ),
            (
                join!("sub_dir_a" / "existing_link"),
                ExistingExpected::Link(OsStr::new("other_file").to_os_string())
            )
        ])
    )?);
    Ok(())
}

#[test]
fn test_extract_existing_content_warn() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });
    let file_content = std::fs::read(join!(extract_dir / "sub_dir_a" / "existing_file"))?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=warn"
    )
    .check_output(
        Some(b""),
        Some(&format_bytes!(
            b"File {} already exists.\nLink {} already exists.\n",
            join!(extract_dir / "sub_dir_a" / "existing_file")
                .to_str()
                .unwrap()
                .as_bytes(),
            join!(extract_dir / "sub_dir_a" / "existing_link")
                .to_str()
                .unwrap()
                .as_bytes()
        )),
    );
    assert!(tree_diff(
        extract_dir,
        tmp_source_dir,
        ExceptionDiffer::from([
            (
                join!("sub_dir_a" / "existing_file"),
                ExistingExpected::Content(file_content)
            ),
            (
                join!("sub_dir_a" / "existing_link"),
                ExistingExpected::Link(OsStr::new("other_file").to_os_string())
            )
        ])
    )?);
    Ok(())
}

#[test]
fn test_extract_existing_content_newer_true() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    // File is modified far before arx created, so we should overwrite
    filetime::set_file_mtime(
        join!(extract_dir / "sub_dir_a" / "existing_file"),
        filetime::FileTime::from_unix_time(0, 0),
    )?;
    filetime::set_symlink_file_times(
        join!(extract_dir / "sub_dir_a" / "existing_link"),
        filetime::FileTime::from_unix_time(0, 0),
        filetime::FileTime::from_unix_time(0, 0),
    )?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=newer"
    )
    .check_output(Some(b""), Some(b""));
    assert!(tree_diff(extract_dir, tmp_source_dir, SimpleDiffer::new())?);
    Ok(())
}

#[test]
fn test_extract_existing_content_newer_false() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    // File is created after source, so we should not overwrite
    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    let file_content = std::fs::read(join!(extract_dir / "sub_dir_a" / "existing_file"))?;

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=newer"
    )
    .check_output(Some(b""), Some(b""));
    assert!(tree_diff(
        extract_dir,
        tmp_source_dir,
        ExceptionDiffer::from([
            (
                join!("sub_dir_a" / "existing_file"),
                ExistingExpected::Content(file_content)
            ),
            (
                join!("sub_dir_a" / "existing_link"),
                ExistingExpected::Link(OsStr::new("other_file").to_os_string())
            )
        ])
    )?);
    Ok(())
}

#[test]
fn test_extract_existing_content_overwrite() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=overwrite"
    )
    .check_output(Some(b""), Some(b""));
    assert!(tree_diff(extract_dir, tmp_source_dir, SimpleDiffer::new())?);
    Ok(())
}

#[test]
fn test_extract_existing_content_error() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_file = BASE_ARX_FILE.path();

    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    cmd!(
        "arx",
        "extract",
        &arx_file,
        "-C",
        extract_dir.path(),
        "--overwrite=error"
    )
    .check_fail(
        b"",
        &format_bytes!(
            b"Error : File {} already exists.\n",
            join!(extract_dir / "sub_dir_a" / "existing_file")
                .to_str()
                .unwrap()
                .as_bytes()
        ),
    );
    assert!(!tree_diff(
        extract_dir,
        tmp_source_dir,
        SimpleDiffer::new()
    )?);
    Ok(())
}
