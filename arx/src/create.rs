use anyhow::{Context, Result};
use std::cell::Cell;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Create an archive.
#[derive(clap::Args)]
pub struct Options {
    /// Archive name to create
    #[arg(
        short = 'f',
        long = "file",
        value_parser,
        required_unless_present("list_compressions")
    )]
    outfile: Option<PathBuf>,

    /// Remove STRIP_PREFIX from the entries' name added to the archive.
    #[arg(long, required = false)]
    strip_prefix: Option<PathBuf>,

    /// Move to BASE_DIR before starting adding content to arx archive.
    #[arg(short = 'C', required = false)]
    base_dir: Option<PathBuf>,

    /// Input files/directories
    #[arg(value_parser, group = "input")]
    infiles: Vec<PathBuf>,

    /// Get the list of files/directories to add from the FILE_LIST (incompatible with INFILES)
    #[arg(short = 'L', long = "file-list", group = "input")]
    file_list: Option<PathBuf>,

    /// Recurse in directories
    #[arg(short, long, required = false, default_value_t = false, action)]
    recurse: bool,

    #[command(flatten)]
    concat_mode: Option<ConcatMode>,

    #[arg(short,long, value_parser=compression_arg_parser, required=false, default_value = "zstd")]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,
}

fn list_compressions() {
    println!("Available compressions :");
    println!(" - None");
    #[cfg(feature = "lz4")]
    println!(" - lz4 (level 0->15)");
    #[cfg(feature = "lzma")]
    println!(" - lzma (level 0->9)");
    #[cfg(feature = "zstd")]
    println!(" - zstd (level -22->22)")
}

fn compression_arg_parser(s: &str) -> Result<jbk::creator::Compression, InvalidCompression> {
    let mut iter = s.splitn(2, '=');
    let compression = iter.next().unwrap().to_ascii_lowercase();
    let level = iter.next();
    Ok(match compression.as_str() {
        "none" => jbk::creator::Compression::None,
        #[cfg(feature = "lz4")]
        "lz4" => match level {
            None => jbk::creator::Compression::lz4(),
            Some(l) => jbk::creator::Compression::Lz4(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        #[cfg(feature = "lzma")]
        "lzma" => match level {
            None => jbk::creator::Compression::lzma(),
            Some(l) => jbk::creator::Compression::Lzma(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        #[cfg(feature = "zstd")]
        "zstd" => match level {
            None => jbk::creator::Compression::zstd(),
            Some(l) => jbk::creator::Compression::Zstd(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        _ => return Err(InvalidCompression::Algorithm(compression)),
    })
}

#[derive(Debug)]
enum InvalidCompression {
    Level(String),
    Algorithm(String),
}

impl fmt::Display for InvalidCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Level(e) => write!(f, "Invalid compression level: {}", e),
            Self::Algorithm(e) => write!(f, "Invalid compression algorithm: {}", e),
        }
    }
}
impl std::error::Error for InvalidCompression {}

#[derive(clap::Args)]
#[group(required = false, multiple = false)]
struct ConcatMode {
    #[arg(short = '1', long, required = false, default_value_t = false, action)]
    /// Create only one file
    one_file: bool,

    #[arg(short = '2', long, required = false, default_value_t = false, action)]
    /// Create two files (a content pack and other)
    two_files: bool,

    #[arg(short = 'N', long, required = false, default_value_t = false, action)]
    /// Create mulitples files (one per pack)
    multiple_files: bool,
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

pub fn create(options: Options, verbose_level: u8) -> Result<()> {
    if options.list_compressions {
        list_compressions();
        return Ok(());
    }

    if verbose_level > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let strip_prefix = match &options.strip_prefix {
        Some(s) => s.clone(),
        None => PathBuf::new(),
    };

    let out_file = std::env::current_dir()?.join(options.outfile.as_ref().unwrap());

    let concat_mode = match &options.concat_mode {
        None => arx::create::ConcatMode::OneFile,
        Some(opt) => {
            let (one, two, multiple) = (opt.one_file, opt.two_files, opt.multiple_files);
            match (one, two, multiple) {
                (true, _, _) => arx::create::ConcatMode::OneFile,
                (_, true, _) => arx::create::ConcatMode::TwoFiles,
                (_, _, true) => arx::create::ConcatMode::NoConcat,
                _ => unreachable!(),
            }
        }
    };

    let jbk_progress = Arc::new(ProgressBar::new());
    let progress = Rc::new(CachedSize::new());
    let mut creator = arx::create::SimpleCreator::new(
        &out_file,
        concat_mode,
        jbk_progress,
        Rc::clone(&progress) as Rc<dyn jbk::creator::CacheProgress>,
        options.compression,
    )?;

    let files_to_add = get_files_to_add(&options)?;

    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };

    let mut fs_adder = arx::create::FsAdder::new(&mut creator, strip_prefix);
    for infile in files_to_add {
        fs_adder.add_from_path(&infile, options.recurse)?;
    }

    let ret = creator.finalize(&out_file);
    println!("Saved place is {}", progress.0.get());
    Ok(ret?)
}
