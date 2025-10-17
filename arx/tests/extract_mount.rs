mod utils;

use rustest::{test, *};

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
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

#[fixture(scope=global)]
fn BaseArxFile(source_dir: SharedTestDir) -> TmpArx {
    let source_dir = source_dir.path();
    let tmp_arx_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_arx = tmp_arx_dir.path().join("test.arx");
    cmd!(
        "arx",
        "create",
        "--outfile",
        &tmp_arx,
        source_dir,
        "--dir-as-root"
    )
    .check_output(Some(""), Some(""));
    TmpArx::new(tmp_arx_dir, tmp_arx)
}

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_mount(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let mount_point = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))?;
    let arx = arx::Arx::new(arx_file.path())?;
    let arxfs = arx::ArxFs::new(arx)?;
    let _mount_handle = arxfs.spawn_mount("Test mounted arx".into(), mount_point.path())?;
    assert!(tree_diff(
        mount_point,
        source_dir.path(),
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[cfg(test)]
macro_rules! tear_down {
    ($name:ident, $function:expr) => {
        struct $name<F>(Option<F>)
        where
            F: FnOnce();
        impl<F> Drop for $name<F>
        where
            F: FnOnce(),
        {
            fn drop(&mut self) {
                self.0.take().unwrap()()
            }
        }
        let _tear_down = $name(Some($function));
    };
}

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_mount_subdir(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let mount_point = tempfile::TempDir::with_prefix_in("mount_", env!("CARGO_TARGET_TMPDIR"))?;

    let mut command = cmd!(
        "arx",
        "mount",
        arx_file.path(),
        "--root-dir",
        "sub_dir_a",
        mount_point.path()
    );
    let status = command.status()?;
    assert!(status.success());
    tear_down!(Unmount, || {
        cmd!("umount", mount_point.path()).status().unwrap();
    });

    // Wait a bit that mount point has been actually setup.
    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut source_sub_dir = source_dir.path().to_path_buf();
    source_sub_dir.push("sub_dir_a");

    assert!(tree_diff(
        &mount_point,
        source_sub_dir,
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_extract(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let extract_dir = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))?;
    arx::extract_all(
        arx_file.path(),
        extract_dir.path(),
        false,
        arx::Overwrite::Error,
    )?;
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[test]
fn test_extract_same_dir(arx_file: BaseArxFile) -> Result {
    // This test that everything go "fine" when extracting an archive in the source directory.
    // But here we don't want to take the risk to polute our source directory shared with other tests.
    // So we extract twice the same archive in the same place.

    let arx_file = arx_file.path();

    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;
    cmd!("arx", "extract", arx_file, "-C", extract_dir.path()).check_output(Some(""), Some(""));
    cmd!("arx", "extract", arx_file, "-C", extract_dir.path()).check_output(Some(""), None);
    Ok(())
}

#[test]
fn test_extract_filter(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;
    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--glob",
        "sub_dir_a/**"
    )
    .check_output(Some(""), Some(""));

    let source_sub_dir = join!((source_dir.path()) / "sub_dir_a");
    let extract_sub_dir = join!(extract_dir / "sub_dir_a");

    assert!(tree_diff(
        extract_sub_dir,
        source_sub_dir,
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[test]
fn test_extract_subdir(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "--root-dir",
        "sub_dir_a",
        "-C",
        extract_dir.path()
    )
    .check_output(Some(""), Some(""));

    let source_sub_dir = join!((source_dir.path()) / "sub_dir_a");

    assert!(tree_diff(extract_dir, source_sub_dir, SimpleDiffer::new())?);
    Ok(())
}

#[test]
fn test_extract_subfile(arx_file: BaseArxFile) -> Result {
    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "--root-dir",
        "sub_dir_a/file1.txt",
        "-C",
        extract_dir.path()
    )
    .check_fail("", "Error : sub_dir_a/file1.txt must be a directory\n");
    Ok(())
}

#[test]
fn test_extract_existing_content_skip(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
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
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=skip"
    )
    .check_output(Some(""), Some(""));
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
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
fn test_extract_existing_content_warn(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
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
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=warn"
    )
    .check_output(
        Some(""),
        Some(&format!(
            "File {} already exists.\nLink {} already exists.\n",
            join!(extract_dir / "sub_dir_a" / "existing_file")
                .to_str()
                .unwrap(),
            join!(extract_dir / "sub_dir_a" / "existing_link")
                .to_str()
                .unwrap()
        )),
    );
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
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
fn test_extract_existing_content_newer_true(
    source_dir: SharedTestDir,
    arx_file: BaseArxFile,
) -> Result {
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
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=newer"
    )
    .check_output(Some(""), Some(""));
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[test]
fn test_extract_existing_content_newer_false(
    source_dir: SharedTestDir,
    arx_file: BaseArxFile,
) -> Result {
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
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=newer"
    )
    .check_output(Some(""), Some(""));
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
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
fn test_extract_existing_content_overwrite(
    source_dir: SharedTestDir,
    arx_file: BaseArxFile,
) -> Result {
    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=overwrite"
    )
    .check_output(Some(""), Some(""));
    assert!(tree_diff(
        extract_dir,
        source_dir.path(),
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[test]
fn test_extract_existing_content_error(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let extract_dir = temp_tree!(0, {
        dir "sub_dir_a" {
            text "existing_file" 100,
            link "existing_link" -> "other_file"
        }
    });

    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "-C",
        extract_dir.path(),
        "--overwrite=error"
    )
    .check_fail(
        "",
        &format!(
            "Error : File {} already exists.\n",
            join!(extract_dir / "sub_dir_a" / "existing_file")
                .to_str()
                .unwrap()
        ),
    );
    assert!(!tree_diff(
        extract_dir,
        source_dir.path(),
        SimpleDiffer::new()
    )?);
    Ok(())
}

#[test]
fn test_extract_subdir_filter(source_dir: SharedTestDir, arx_file: BaseArxFile) -> Result {
    let extract_dir = tempfile::TempDir::with_prefix_in("extract_", env!("CARGO_TARGET_TMPDIR"))?;

    cmd!(
        "arx",
        "extract",
        arx_file.path(),
        "--root-dir",
        "sub_dir_a",
        "-C",
        extract_dir.path(),
        "--glob",
        "*.txt"
    )
    .check_output(Some(""), Some(""));

    let source_sub_dir = join!((source_dir.path()) / "sub_dir_a");

    struct DiffOnlyTxt(bool);

    impl Differ for DiffOnlyTxt {
        type Result = bool;

        fn result(self) -> Self::Result {
            self.0
        }

        fn push(&mut self, _entry_name: &OsStr) {}

        fn pop(&mut self) {}

        fn equal(&mut self, entry_tested: &TreeEntry, _entry_ref: &TreeEntry) {
            // Only txt should be equal
            if !entry_tested.file_name().to_string_lossy().ends_with(".txt") {
                self.0 = false
            }
        }

        fn added(&mut self, _entry: &TreeEntry) {
            // We don't want any extra file
            self.0 = false
        }

        fn removed(&mut self, entry: &TreeEntry) {
            // We care only about txt file
            if entry.file_name().to_string_lossy().ends_with(".txt") {
                self.0 = false
            }
        }

        fn diff(&mut self, _entry_tested: &TreeEntry, _entry_ref: &TreeEntry) {
            // Existing file must match
            self.0 = false
        }
    }

    assert!(tree_diff(extract_dir, source_sub_dir, DiffOnlyTxt(true))?);
    Ok(())
}

#[rustest::main]
fn main() {}
