use std::path::{Path, PathBuf};
use std::process::Command;

// Generate a fake directory with fake content.
fn spawn_mount() -> std::io::Result<(arx_test_dir::BackgroundSession, PathBuf)> {
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

#[test]
fn test_create() {
    let (_source_mount_handle, source_mount_point) = spawn_mount().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_arx");
    let arx_file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test.arx");
    let output = Command::new(bin_path)
        .arg("--verbose")
        .arg("create")
        .arg("--file")
        .arg(&arx_file)
        .arg("-r1")
        .arg("--strip-prefix")
        .arg(&source_mount_point)
        .arg(&source_mount_point)
        .output()
        .expect("foo");
    println!("Out : {}", String::from_utf8(output.stdout).unwrap());
    println!("Err : {}", String::from_utf8(output.stderr).unwrap());
    assert!(output.status.success());
    assert!(arx_file.is_file());

    let mount_point = tempfile::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let arx = libarx::Arx::new(arx_file).unwrap();
    let arxfs = libarx::ArxFs::new(arx).unwrap();
    let _mount_handle = arxfs.spawn_mount(mount_point.path()).unwrap();
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
