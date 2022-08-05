mod create;

use jubako as jbk;

use clap::Parser;
use create::{Creator, Entry};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
struct Cli {
    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Archive name to create
    #[clap(short, long, value_parser)]
    outfile: PathBuf,

    // verbose
    #[clap(short, long, action=clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> jbk::Result<()> {
    let cli = Cli::parse();

    if cli.verbose > 0 {
        println!("Creating archive {:?}", cli.outfile);
        println!("With files {:?}", cli.infiles);
    }

    let mut creator = Creator::new(&cli.outfile);

    creator.start()?;
    for infile in cli.infiles {
        creator.push_back(Entry::new(infile)?);
    }

    creator.run()?;

    creator.finalize(cli.outfile)?;

    Ok(())
}
