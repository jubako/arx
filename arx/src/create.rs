use anyhow::{anyhow, Context, Result};
use color_print::cstr;
use log::{debug, info};
use std::cell::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{absolute, Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use clap::{Parser, ValueHint};

const AFTER_HELP: &str = cstr!(
    "
<s,u>Specifying input files:</>

<s>From command line ([INFILES])</s>

The command format is :
<i>$ arx -o my_archive.arx input_file1 input_file2 input_dir1 ...</i>

<s>From list file ([FILE_LIST])</s>

The command format is :
<i>$ arx -o my_archive.arx -L file_list.txt</i>

File list can be generated this way :
<i>$ find input_dir > file_list.txt</i>

You can apply filters on this list if you want to.

<s>Tree structure and directory creation</s>

Arx store files using a tree structure of Directory/File/Link so a file <s>foo/bar.txt</s> cannot
exists without a directory <s>foo</s>.

When adding <i>foo</i> directory recursively, both <i>foo</i> and <i>foo/bar.txt</i> are added
using attribute from filesystem metadata (stats on Linux)
When directly adding <i>foo/bar.txt</i>, <i>foo</i> directory is automatically created using default
metadata:
- owner and group: 1000,
- right: 0x755,
- mtime: 0

This is the same thing when using file list. It is preferable to list both directories and files in the listing:
```
foo
foo/bar.txt
```
(If you've used <i>find</i> to generate the list : don't use <i>-type f</i> option)

<s>About <i>--follow-symlink</i> option</s>

When passing entries to arx, you must be consistent with <i>--follow-symlink</i>.
One way to be inconsistent is:
1. You have a link (L), pointing to a directory (D) containing a file (D/F).
2. You create a arx passing as input both L and L/F.
3. You don't give <i>--follow-symlink</i>
4. Arx create a symlink L and then try to add the file F to directory L, which is not possible
   because L is a symlink.

<s>Triming path</s>

By default, arx trim input path and remove any parent parts. This can be changed with the <i>--keep-parents</i>
option, in this case the path given as input is fully add to the archive.

Absolute path are always trimmed of its first part to become a relative path.

The option <i>--dir_as_root</i> makes arx trim the directory itself. So (if recursion is activated) all entries
in the given directory as place at root of the created archive.
This option has no effect if the input path is a file.

<s,u>Compression detection/selection:</>

Arx automatically detect if a content should be compressed or not based on a heuristic using
Shannon entropy.
You can select which compression algorithm is used using <i>--compression</i> option, either
<i>--compression <<algorithm></i> or <i>--compression <<algorithm>=<<level></i>.
List of available compression algorithms can be obtained using <i>--list-compressions</i> option.
You can use <i>--compresssion none</i> to deactivate compression.
It is not possible to force compression of content (patch welcome).
"
);

const USAGE: &str = cstr!("<s>arx create</s> -o archive.arx [OPTIONS] [INFILES]...");

/// Create an archive.
#[derive(Parser, Debug)]
#[command(after_long_help=AFTER_HELP, override_usage=USAGE)]
pub struct Options {
    /// File path of the archive to create.
    ///
    /// Relative path are relative to the current working dir.
    /// `BASE_DIR` option is used after resolving relative path.
    #[arg(
        short,
        long,
        value_parser,
        required_unless_present("list_compressions"),
        value_hint=ValueHint::FilePath,
    )]
    outfile: Option<jbk::Utf8PathBuf>,

    /// Move to BASE_DIR before starting adding content to arx archive.
    ///
    /// Argument `INFILES` or `STRIP_PREFIX` must be relative to `BASE_DIR`.
    /// `OUTFILE` and `FILE_LIST` path is always relative to current directory.
    /// Paths listed in `FILE_LIST` are related to `BASED_DIR`
    #[arg(short = 'C', required = false, value_hint=ValueHint::DirPath, verbatim_doc_comment, help_heading="Input options")]
    base_dir: Option<PathBuf>,

    /// Keep N parents from given path.
    ///
    /// If N > number of parent in the path, keep the path full.
    #[arg(short = 'k', long, required = false, default_value_t = Default::default())]
    keep_parents: bool,

    /// Input dir are considered as root directory.
    ///
    /// Directories given in input are considered as root directory.
    /// This means we add file/directory in the given directory directly in the root of arx archive.
    /// If input is a file, this options has no effect.
    /// If several directories are given, they are all considered as root. However, as it is not possible to have
    /// duplicated entries in an arx achive, the directories must be disjoint.
    #[arg(long, required = false, default_value_t = false)]
    dir_as_root: bool,

    /// Input files/directories
    ///
    /// This is an option incompatible with `FILE_LIST.`
    ///
    /// In this mode `recurse` is true by default.
    /// Use `--no-recurse` to avoid recursion.
    ///
    /// Arx is storing only relative path. If INFILES contains absolute paths, root
    /// prefix is removed.
    #[arg(value_parser, group = "input", value_hint=ValueHint::AnyPath, help_heading="Input options")]
    infiles: Vec<PathBuf>,

    /// Get the list of files/directories to add from the FILE_LIST (incompatible with INFILES)
    ///
    /// This is an option incompatible with `INFILES`.
    ///
    /// Relative path are relative to the current working dir. `BASE_DIR` option is used after resolving relative path.
    ///
    /// In this mode, `recurse` is false by default.
    /// This allow FILE_LIST listing both the directory and (subset of) files in the given directory.
    /// Use `--recurse` to activate recursion.
    #[arg(short = 'L', long = "file-list", group = "input", verbatim_doc_comment, value_hint=ValueHint::FilePath, help_heading="Input options")]
    file_list: Option<PathBuf>,

    /// Recurse in directories
    ///
    /// Default value is true if `INFILES` is passed and false is `FILE_LIST` is passed.
    #[arg(
        short,
        long,
        required = false,
        default_value_t = true,
        default_value_if("no_recurse", clap::builder::ArgPredicate::IsPresent, "false"),
        conflicts_with = "no_recurse",
        action,
        help_heading = "Input options"
    )]
    recurse: bool,

    /// Force `--recurse` to be false.
    #[arg(long, help_heading = "Input options")]
    no_recurse: bool,

    #[command(flatten)]
    concat_mode: Option<jbk::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(short,long, value_parser=jbk::cmd_utils::compression_arg_parser, required=false, default_value = "zstd")]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,

    /// Print a progression of the creation
    #[arg(long, default_value_t = false, action)]
    progress: bool,

    /// Overwrite existing archive file
    #[arg(short, long, required = false, default_value_t = false, action)]
    force: bool,

    /// Follow symbolic link found in the input files
    #[arg(
        long,
        required = false,
        default_value_t = false,
        action,
        help_heading = "Input options"
    )]
    follow_symlink: bool,

    #[arg(from_global)]
    verbose: u8,
}

fn check_input_paths_exist(file_list: &[PathBuf]) -> Result<()> {
    // Check that input files actually exists
    for file in file_list.iter() {
        if !file.exists() {
            return Err(anyhow!(
                "Input {} path doesn't exist or cannot be accessed",
                file.display()
            ));
        }
    }
    Ok(())
}

fn check_output_path_writable(out_file: &Path, force: bool) -> Result<()> {
    let out_file = absolute(out_file)?;
    if !out_file.parent().unwrap().is_dir() {
        Err(anyhow!(
            "Directory {} doesn't exist",
            out_file.parent().unwrap().display()
        ))
    } else if out_file.exists() && !force {
        Err(anyhow!(
            "File {} already exists. Use option --force to overwrite it.",
            out_file.display()
        ))
    } else {
        Ok(())
    }
}

struct ProgressBar {
    comp_clusters: indicatif::ProgressBar,
    uncomp_clusters: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new() -> Self {
        let style = indicatif::ProgressStyle::with_template(
            "{prefix} : {wide_bar:.cyan/blue} {pos:4} / {len:4}",
        )
        .unwrap()
        .progress_chars("#+-");
        let multi = indicatif::MultiProgress::new();
        let comp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style.clone())
            .with_prefix("Compressed Cluster  ");
        let uncomp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style)
            .with_prefix("Uncompressed Cluster");
        multi.add(comp_clusters.clone());
        multi.add(uncomp_clusters.clone());
        Self {
            comp_clusters,
            uncomp_clusters,
        }
    }
}

impl jbk::creator::Progress for ProgressBar {
    fn new_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc_length(1)
    }
    fn handle_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc(1)
    }
}

struct CachedSize(Cell<u64>);

impl jbk::creator::CacheProgress for CachedSize {
    fn cached_data(&self, size: jbk::Size) {
        self.0.set(self.0.get() + size.into_u64());
    }
}

impl CachedSize {
    fn new() -> Self {
        Self(Cell::new(0))
    }
}

pub fn create(options: Options) -> Result<()> {
    if options.list_compressions {
        jbk::cmd_utils::list_compressions();
        return Ok(());
    }

    let out_file = options.outfile.as_ref().expect(
        "Clap unsure it is Some, except if we have list_compressions, and so we return early",
    );
    check_output_path_writable(out_file.as_std_path(), options.force)?;

    info!("Creating archive {:?}", out_file);
    let file_list = options
        .file_list
        .as_ref()
        .map(std::path::absolute)
        .transpose()?;
    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };

    let jbk_progress: Arc<dyn jbk::creator::Progress> = if options.progress {
        Arc::new(ProgressBar::new())
    } else {
        Arc::new(())
    };
    let cache_progress = Rc::new(CachedSize::new());
    let mut creator = arx::create::SimpleCreator::new(
        out_file,
        match options.concat_mode {
            None => jbk::creator::ConcatMode::OneFile,
            Some(e) => e.into(),
        },
        jbk_progress,
        cache_progress.clone(),
        options.compression,
    )?;

    let mut adder = arx::create::FsAdder::new(
        &mut creator,
        options.keep_parents,
        options.follow_symlink,
        options.dir_as_root,
    );

    if let Some(file_list) = file_list {
        let file = File::open(&file_list)
            .with_context(|| format!("Cannot open {}", file_list.display()))?;
        let files_list = BufReader::new(file)
            .lines()
            .map(|l| -> Result<PathBuf> { Ok(l?.into()) })
            .collect::<Result<Vec<_>>>()?;
        adder.add_from_list(files_list.into_iter())?;
    } else {
        check_input_paths_exist(&options.infiles)?;
        for infile in options.infiles {
            debug!("Adding file {infile:?}");
            adder.add_from_path(&infile, options.recurse)?;
        }
    };

    let ret = creator.finalize();
    debug!("Saved place is {}", cache_progress.0.get());
    Ok(ret?)
}
