use anyhow::{Context, Result};
use log::{debug, info};
use std::cell::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Create an archive.
#[derive(clap::Args, Debug)]
pub struct Options {
    /// File path of the archive to create.
    ///
    /// Relative path are relative to the current working dir.
    /// `BASE_DIR` option is used after resolving relative path.
    #[arg(
        short = 'f',
        long = "file",
        value_parser,
        required_unless_present("list_compressions")
    )]
    outfile: Option<PathBuf>,

    /// Remove STRIP_PREFIX from the entries' name added to the archive.
    #[arg(long, required = false)]
    strip_prefix: Option<arx::PathBuf>,

    /// Move to BASE_DIR before starting adding content to arx archive.
    ///
    /// Argument `INFILES` or `STRIP_PREFIX` must be relative to `BASE_DIR`.
    #[arg(short = 'C', required = false)]
    base_dir: Option<PathBuf>,

    /// Input files/directories
    ///
    /// This is an option incompatible with `FILE_LIST.`
    ///
    /// In this mode `recurse` is true by default.
    /// Use `--no-recurse` to avoid recursion.
    #[arg(value_parser, group = "input")]
    infiles: Vec<PathBuf>,

    /// Get the list of files/directories to add from the FILE_LIST (incompatible with INFILES)
    ///
    /// This is an option incompatible with `INFILES`.
    ///
    /// In this mode, `recurse` is false by default.
    /// This allow FILE_LIST listing both the directory and (subset of) files in the given directory.
    /// Use `--recurse` to activate recursion.
    #[arg(short = 'L', long = "file-list", group = "input", verbatim_doc_comment)]
    file_list: Option<PathBuf>,

    /// Recurse in directories
    ///
    /// Default value is true if `INFILES` is passed and false is `FILE_LIST` is passed.
    #[arg(
        short,
        long,
        required = false,
        default_value_t = false,
        default_value_ifs([
            ("no_recurse", clap::builder::ArgPredicate::IsPresent, "false"),
            ("infiles", clap::builder::ArgPredicate::IsPresent, "true")
        ]),
        conflicts_with = "no_recurse",
        action
    )]
    recurse: bool,

    /// Force `--recurse` to be false.
    #[arg(long)]
    no_recurse: bool,

    #[command(flatten)]
    concat_mode: Option<arx::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(short,long, value_parser=arx::cmd_utils::compression_arg_parser, required=false, default_value = "zstd")]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,

    #[arg(from_global)]
    verbose: u8,
}

fn get_files_to_add(options: &Options) -> Result<Vec<PathBuf>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list).with_context(|| format!("Cannot open {:?}", file_list))?;
        let mut files = Vec::new();
        for line in BufReader::new(file).lines() {
            files.push(line?.into());
        }
        Ok(files)
    } else {
        Ok(options.infiles.clone())
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
        arx::cmd_utils::list_compressions();
        return Ok(());
    }

    info!("Creating archive {:?}", options.outfile);
    info!("With files {:?}", options.infiles);

    let strip_prefix = match &options.strip_prefix {
        Some(s) => s.clone(),
        None => arx::PathBuf::new(),
    };

    let out_file = std::env::current_dir()?.join(options.outfile.as_ref().unwrap());
    let files_to_add = get_files_to_add(&options)?;

    let jbk_progress = Arc::new(ProgressBar::new());
    let progress = Rc::new(CachedSize::new());
    let mut creator = arx::create::SimpleCreator::new(
        &out_file,
        options.concat_mode.into(),
        jbk_progress,
        Rc::clone(&progress) as Rc<dyn jbk::creator::CacheProgress>,
        options.compression,
    )?;

    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };

    let mut fs_adder = arx::create::FsAdder::new(&mut creator, strip_prefix);
    for infile in files_to_add {
        fs_adder.add_from_path(&infile, options.recurse)?;
    }

    let ret = creator.finalize(&out_file);
    debug!("Saved place is {}", progress.0.get());
    Ok(ret?)
}
