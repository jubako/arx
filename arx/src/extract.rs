use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Extract the content of an archive
#[derive(clap::Args)]
pub struct Options {
    /// Archive to read
    #[clap(short = 'f', long = "file")]
    infile: PathBuf,

    /// Directory in which extract the archive. (Default to current directory)
    #[clap(short = 'C', required = false)]
    outdir: Option<PathBuf>,

    /// Files to extract
    #[clap(value_parser)]
    extract_files: Vec<PathBuf>,

    /// Print a progress bar of the extraction
    #[clap(short = 'p', long = "progress", default_value_t = false, action)]
    progress: bool,

    /// Get the list of files/directories to extract from the FILE_LIST (incompatible with EXTRACT_FILES)
    #[clap(short = 'L', long = "file-list")]
    file_list: Option<PathBuf>,
}

fn get_files_to_extract(options: &Options) -> jbk::Result<HashSet<PathBuf>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list)?;
        let mut files: HashSet<PathBuf> = Default::default();
        for line in BufReader::new(file).lines() {
            files.insert(line?.into());
        }
        Ok(files)
    } else {
        Ok(options.extract_files.iter().cloned().collect())
    }
}

pub fn extract(options: Options, verbose_level: u8) -> jbk::Result<()> {
    let files_to_extract = get_files_to_extract(&options)?;
    let outdir = match options.outdir {
        Some(o) => o,
        None => current_dir()?,
    };
    if verbose_level > 0 {
        println!("Extract archive {:?} in {:?}", &options.infile, outdir);
    }
    arx::extract(&options.infile, &outdir, files_to_extract, options.progress)
}
