use std::{io::Read, sync::LazyLock};

use rand::prelude::*;
use std::fs::{create_dir, write};
pub use std::path::Path;

#[cfg(unix)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(path: P, target: Q) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, path)
}
#[cfg(unix)]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(path: P, target: Q) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, path)
}

#[cfg(windows)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(path: P, target: Q) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, path)
}
#[cfg(windows)]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(path: P, target: Q) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, path)
}

struct BinRead<'a>(pub &'a mut SmallRng);

impl Read for BinRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let to_read_len = std::cmp::min(buf.len(), 1024);
        self.0.fill_bytes(&mut buf[..to_read_len]);
        Ok(to_read_len)
    }
}

/// Generate a tree of directory/file/link
///
/// This macro is intended to be used as a function.
/// The first argument is a integer used as seed to generate random numbers/content
/// The second argument is a description of the tree to create.
/// It format is `{ instruction, instruction, ... }`
/// Each instruction can be :
/// - `dir "<name>" { inner_instructions }`:
///   This will create a subdir `<name>` with the content `inner_instructions`
/// - `text "<name>" <len>`: Generate a text file `<name>` of size `<len>`
/// - `bin "<name>" <len>`: Generate a binary file `<name>` of size `<len>`
/// - `link` "<name>" -> "<target>": Generate a symlink `<name>` to file `<target>`
/// - `link_din` "<name>" -> "<target>": Generate a symlink `<name>` to a directory `<target>`
/// - `loop <count> { instructions }`: Repeat `instuctions` `<count>` times
///
/// All numbers (count/len) can be a simple literal (`42`) or a range (`1..42`).
/// If it is a range, the actual number is a random number in this range.
///
/// `loop` instruction generate a context (the current increment of the loop) which MUST
/// be used in "<name>". By default, the context is name `ctx`.
/// So in a loop, all "<name>" or "<target>" must be in the form of "foo_{ctx}_bar".
/// It is possible to give a explicit name to the context with `loop my_name=42 { instructions }`.
/// Loop can be neested. As all contexts MUST be used, you MUST explicit name your contexts
/// to avoid conflict.
macro_rules! temp_tree {
    // This macro is implemented using:
    // - tt_muncher (https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html)
    //   to parse the input structure
    // - internal_rules (https://veykril.github.io/tlborm/decl-macros/patterns/internal-rules.html)
    //   to handle instructions and "sub routine"


    // -----------
    // Entry point
    // -----------
    ($seed:literal, { $($what:tt)* }) => {
        {
            let temp_path =
            tempfile::TempDir::with_prefix_in("source_", env!("CARGO_TARGET_TMPDIR")).unwrap();
            let mut rng = SmallRng::seed_from_u64($seed);
            temp_tree!(@instr, temp_path.path(), rng, [], $($what)*);
            Ok(temp_path)
        }
    };

    // -----------------------
    // Parsing of instructions
    // -----------------------
    // End of instruction
    (@instr, $path:expr, $rng:ident, $context:tt,) => {};

    // Handle dir instruction
    (@instr, $path:expr, $rng:ident, $context:tt, dir $sub_path:tt $what:tt ) => {
        temp_tree!(@dir, $path, $rng, $context, $sub_path, $what);
    };
    (@instr, $path:expr, $rng:ident, $context:tt, dir $sub_path:tt $what:tt, $($left:tt)* ) => {
        temp_tree!(@dir, $path, $rng, $context, $sub_path, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle text instruction
    (@instr, $path:expr, $rng:ident, $context:tt, text $sub_path:tt $what:tt ) => {
        temp_tree!(@text, $path, $rng, $context, $sub_path, $what);
    };
    (@instr, $path:expr, $rng:ident, $context:tt, text $sub_path:tt $what:tt, $($left:tt)* ) => {
        temp_tree!(@text, $path, $rng, $context, $sub_path, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle binary instruction
    (@instr, $path:expr, $rng:ident, $context:tt, bin $sub_path:tt $what:tt ) => {
        temp_tree!(@bin, $path, $rng, $context, $sub_path, $what);
    };

    (@instr, $path:expr, $rng:ident, $context:tt, bin $sub_path:tt $what:tt, $($left:tt)* ) => {
        temp_tree!(@bin, $path, $rng, $context, $sub_path, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle symlink instruction
    (@instr, $path:expr, $rng:ident, $context:tt, link $sub_path:tt -> $what:tt ) => {
        temp_tree!(@link, $path, $rng, $context, $sub_path, $what);
    };

    (@instr, $path:expr, $rng:ident, $context:tt, link $sub_path:tt -> $what:tt, $($left:tt)* ) => {
        temp_tree!(@link, $path, $rng, $context, $sub_path, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle symlink_dir instruction
    (@instr, $path:expr, $rng:ident, $context:tt, link_dir $sub_path:tt -> $what:tt ) => {
        temp_tree!(@link_dir, $path, $rng, $context, $sub_path, $what);
    };
    (@instr, $path:expr, $rng:ident, $context:tt, link_dir $sub_path:tt -> $what:tt, $($left:tt)* ) => {
        temp_tree!(@link_dir, $path, $rng, $context, $sub_path, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle loop instruction
    (@instr, $path:expr, $rng:ident, $context:tt, loop $nb:tt $what:tt ) => {
        temp_tree!(@loop, $path, $rng, $context, ctx, $nb, $what);
    };
    (@instr, $path:expr, $rng:ident, $context:tt, loop $nb:tt $what:tt, $($left:tt)* ) => {
        temp_tree!(@loop, $path, $rng, $context, ctx, $nb, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // Handle named context gen instruction
    (@instr, $path:expr, $rng:ident, $context:tt, loop $ctx_name:ident=$nb:tt $what:tt ) => {
        temp_tree!(@loop, $path, $rng, $context, $ctx_name, $nb, $what);
    };
    (@instr, $path:expr, $rng:ident, $context:tt, loop $ctx_name:ident=$nb:tt $what:tt, $($left:tt)* ) => {
        temp_tree!(@loop, $path, $rng, $context, $ctx_name, $nb, $what);
        temp_tree!(@instr, $path, $rng, $context, $($left)*)
    };

    // ------------------------
    // Handling of instructions
    // ------------------------

    // Empty dir
    (@dir, $path:expr, $rng:ident, $context:tt, $sub_path:tt, { }) => {
        create_dir($path.join(&temp_tree!(@ctx, $sub_path, $context)))?;
    };
    // Dir with content
    (@dir, $path:expr, $rng:ident, $context:tt, $sub_path:tt, { $($what:tt)+ }) => {
        {
            let new_path = $path.join(&temp_tree!(@ctx, $sub_path, $context));
            create_dir(&new_path)?;
            temp_tree!(@instr, new_path, $rng, $context, $($what)+) ;
        }
    };

    // Text file
    (@text, $path:expr, $rng:ident, $context:tt, $sub_path:tt, $len:tt) => {
        let len = temp_tree!(@num, $rng, $len);
        let data = lipsum::lipsum_words_with_rng(&mut $rng, len);
        write($path.join(&temp_tree!(@ctx, $sub_path, $context)), data.as_bytes())?;
    };


    // Binary file
    (@bin, $path:expr, $rng:ident, $context:tt, $sub_path:tt, $len:tt) => {
        let len = temp_tree!(@num, $rng, $len);
        let data = BinRead(&mut $rng);
        let mut file = std::fs::File::create($path.join(&temp_tree!(@ctx, $sub_path, $context)))?;
        std::io::copy(&mut data.take(len), &mut file)?;
    };

    // Symlink to file
    (@link, $path:expr, $rng:ident, $context:tt, $sub_path:tt, $target:expr) => {
        {
            let sub_path = $path.join(temp_tree!(@ctx, $sub_path, $context));
            let target = temp_tree!(@ctx, $target, $context);
            symlink(sub_path, target)?;
        }
    };

    // Symlink to directory
    (@link_dir, $path:expr, $rng:ident, $context:tt, $sub_path:tt, $target:expr) => {
        {
            let sub_path = $path.join(temp_tree!(@ctx, $sub_path, $context));
            let target = temp_tree!(@ctx, $target, $context);
            symlink_dir(sub_path, target)?;
        }
    };

    // Loop without upper context
    (@loop, $path:expr, $rng:ident, [], $ctx_name:ident, $nb:tt, { $($what:tt)+ }) => {
        for $ctx_name in 0..temp_tree!(@num, $rng, $nb) {
            temp_tree!(@instr, $path, $rng, [ $ctx_name ], $($what)+);
        }
    };
    // Loop with upper context
    (@loop, $path:expr, $rng:ident, [$($context:tt)|+], $ctx_name:ident, $nb:tt, { $($what:tt)+ }) => {
        for $ctx_name in 0..temp_tree!(@num, $rng, $nb) {
            temp_tree!(@instr, $path, $rng, [ $($context)|+ | $ctx_name ], $($what)+);
        }
    };

    // -------
    // Helpers
    // -------

    // Generate a number
    (@num, $rng:ident, ($start:tt..$end:tt)) => {
        $rng.gen_range($start..$end)
    };
    (@num, $rng:ident, $what:expr) => {
        $what
    };
    (@ctx, $path:tt, []) => {
        $path
    };
    (@ctx, $path:tt, [$($c:tt)|+] ) => {
        format!($path, $($c=$c),+)
    };
}

static SHARED_TEST_DIR: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    (|| -> std::io::Result<tempfile::TempDir> {
        temp_tree!(1, {
            dir "sub_dir_a" {
                text "file_2.txt" (500..1000),
                loop  (10..50) { text "file{ctx}.txt" (500..1000) }
            },
            dir "sub_dir_b" {
                loop 10 { bin "file{ctx}.bin" (5000..10000) },
                loop 10 { link "link_to_file{ctx}" -> "file{ctx}.bin" },
            },
            link_dir "sub_dir_link" -> "sub_dir_b",
            dir "empty_sub_dir" {},
            loop dir_ctx=(1..5) {
                dir "gen_sub_dir_{dir_ctx}" {
                    loop (1..10) { text "gen_sub_file_{dir_ctx}_{ctx}" (500..1000)},
                    loop (1..2) { dir "gen_sub_empty_dir_{ctx}{dir_ctx:0}" {} }
                }
            }
        })
    })()
    .unwrap()
});

macro_rules! cmd {
    ("{cmd}", $command:ident, $arg:expr) => {{
        $command.arg($arg);
        $command.output().expect("Launching arx command should work.")
    }};
    ("{cmd}", $command:ident, $arg:expr, $($args:expr),+) => {{
        $command.arg($arg);
        cmd!("{cmd}", $command, $($args),+)
    }};
    ("arx", $sub_command:literal, $($args:expr),*) => {{
        let arx_bin = env!("CARGO_BIN_EXE_arx");
        let mut command = std::process::Command::new(&arx_bin);
        command.env("NO_COLOR", "1");
        cmd!("{cmd}", command, $sub_command, $($args),*)
    }};
    ($prog:literal, $($args:expr),*) => {{
        let mut command = std::process::Command::new($prog);
        cmd!("{cmd}", command, $($args),*)
    }};
}

#[test]
fn test_crate_non_existant_input() {
    use std::path::Path;

    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    use std::path::Path;

    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir
        .path()
        .join("non_existant_directory")
        .join("test.arx");
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
    use std::path::Path;

    let tmp_source_dir = SHARED_TEST_DIR.path();
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
    let arx_tmp_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tempdir should work");
    let arx_file = arx_tmp_dir.path().join("test.arx");
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
