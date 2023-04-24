use jubako as jbk;
use libarx as arx;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Options {
    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Archive name to create
    #[clap(short, long, value_parser)]
    outfile: PathBuf,

    #[clap(short = 'L', long = "file-list")]
    file_list: Option<PathBuf>,

    #[clap(short, long, required = false, default_value_t = false, action)]
    recurse: bool,
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

pub fn create(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let mut creator = arx::create::Creator::new(&options.outfile)?;

    let files_to_add = get_files_to_add(&options)?;
    for infile in files_to_add {
        creator.add_from_path(infile, options.recurse)?;
    }

    creator.finalize(options.outfile)
}
