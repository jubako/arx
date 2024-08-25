#[cfg(all(unix, not(feature = "in_ci")))]
mod inner {
    pub use std::path::{Path, PathBuf};
    pub use std::process::Command;

    // Generate a fake directory with fake content.
    pub fn spawn_mount() -> std::io::Result<(arx_test_dir::BackgroundSession, PathBuf)> {
        let mount_path = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let builder = arx_test_dir::ContextBuilder::new();
        let context = builder.create();
        let dir = arx_test_dir::DirEntry::new_root(context);
        let mount_dir = arx_test_dir::TreeFs::new(dir);
        Ok((
            mount_dir.spawn(mount_path.path())?,
            mount_path.path().into(),
        ))
    }
}

#[cfg(all(unix, not(feature = "in_ci")))]
#[test]
fn test_create() {
    use inner::*;

    let (_source_mount_handle, source_mount_point) = spawn_mount().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_arx");
    let arx_file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test.arx");
    let output = Command::new(bin_path)
        .arg("--verbose")
        .arg("create")
        .arg("--outfile")
        .arg(&arx_file)
        .arg("-C")
        .arg(&source_mount_point.parent().unwrap())
        .arg("--strip-prefix")
        .arg(&source_mount_point.file_name().unwrap())
        .arg(&source_mount_point.file_name().unwrap())
        .output()
        .expect("foo");
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
    let output = Command::new("diff")
        .arg("-r")
        .arg(&source_mount_point)
        .arg(&mount_point.path())
        .output()
        .unwrap();
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err: {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
}
