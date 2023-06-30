use jubako as jbk;
use libarx as arx;
use std::cell::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(clap::Args)]
pub struct Options {
    // Archive name to create
    #[clap(short = 'f', long = "file", value_parser)]
    outfile: PathBuf,

    #[clap(long, required = false)]
    strip_prefix: Option<PathBuf>,

    #[clap(short = 'C', required = false)]
    base_dir: Option<PathBuf>,

    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    #[clap(short = 'L', long = "file-list")]
    file_list: Option<PathBuf>,

    #[clap(short, long, required = false, default_value_t = false, action)]
    recurse: bool,

    #[clap(short = '1', long, required = false, default_value_t = false, action)]
    one_file: bool,
}

fn get_files_to_add(options: &Options) -> jbk::Result<Vec<PathBuf>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list)?;
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

impl arx::create::Progress for CachedSize {
    fn cached_data(&self, size: jbk::Size) {
        self.0.set(self.0.get() + size.into_u64());
    }
}

impl CachedSize {
    fn new() -> Self {
        Self(Cell::new(0))
    }
}

pub fn create(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let strip_prefix = match &options.strip_prefix {
        Some(s) => s.clone(),
        None => PathBuf::new(),
    };

    let out_file = std::env::current_dir()?.join(&options.outfile);

    let concat_mode = if options.one_file {
        arx::create::ConcatMode::OneFile
    } else {
        arx::create::ConcatMode::TwoFiles
    };

    let jbk_progress = Arc::new(ProgressBar::new());
    let progress = Rc::new(CachedSize::new());
    let mut creator = arx::create::Creator::new(
        &out_file,
        strip_prefix,
        concat_mode,
        jbk_progress,
        Rc::clone(&progress) as Rc<dyn arx::create::Progress>,
    )?;

    let files_to_add = get_files_to_add(&options)?;

    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };
    for infile in files_to_add {
        creator.add_from_path(infile, options.recurse)?;
    }

    let ret = creator.finalize(&out_file);
    println!("Saved place is {}", progress.0.get());
    ret
}
