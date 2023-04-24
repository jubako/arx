use jubako as jbk;
use libarx as arx;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Options {
    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Archive name to create
    #[clap(short, long, value_parser)]
    outfile: PathBuf,

    #[clap(short, long, default_value_t = false)]
    recurse: bool,
}

pub fn create(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let mut creator = arx::create::Creator::new(&options.outfile)?;

    for infile in options.infiles {
        creator.add_from_path(infile, options.recurse)?;
    }

    creator.finalize(options.outfile)
}
