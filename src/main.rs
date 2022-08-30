mod create;
mod dump;
mod entry;
mod list;

use jubako as jbk;

use clap::{Args, Parser, Subcommand};
use create::{Creator, Entry};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "arx")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    Create(Create),

    #[clap(arg_required_else_help = true)]
    List(List),

    #[clap(arg_required_else_help = true)]
    Dump(Dump),
}

#[derive(Args)]
struct Create {
    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Archive name to create
    #[clap(short, long, value_parser)]
    outfile: PathBuf,
}

#[derive(Args)]
struct List {
    #[clap(value_parser)]
    infile: PathBuf,
}

#[derive(Args)]
struct Dump {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    path: String,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Create(create_cmd) => {
            if args.verbose > 0 {
                println!("Creating archive {:?}", create_cmd.outfile);
                println!("With files {:?}", create_cmd.infiles);
            }

            let mut creator = Creator::new(&create_cmd.outfile);

            creator.start()?;
            for infile in create_cmd.infiles {
                creator.push_back(Entry::new(infile)?);
            }

            creator.run()?;

            creator.finalize(create_cmd.outfile)
        }

        Commands::List(list_cmd) => {
            if args.verbose > 0 {
                println!("Listing entries in archive {:?}", list_cmd.infile);
            }

            list::list(list_cmd.infile)
        }

        Commands::Dump(dump_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Dump entry {} in archive {:?}",
                    dump_cmd.path, dump_cmd.infile
                );
            }

            dump::dump(dump_cmd.infile, dump_cmd.path)
        }
    }
}
