use jubako as jbk;

use arx::{Creator, Entry};
use clap::{Args, Parser, Subcommand};
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

    #[clap(arg_required_else_help = true)]
    Extract(Extract),

    #[clap(arg_required_else_help = true)]
    Mount(Mount),
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

#[derive(Args)]
struct Extract {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    outdir: PathBuf,
}

#[derive(Args)]
struct Mount {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    mountdir: PathBuf,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Create(create_cmd) => {
            if args.verbose > 0 {
                println!("Creating archive {:?}", create_cmd.outfile);
                println!("With files {:?}", create_cmd.infiles);
            }

            let mut creator = Creator::new(&create_cmd.outfile)?;

            let root_parent = jbk::Vow::new(0.into());
            let root_parent: jbk::Generator<jbk::EntryIdx, u64> =
                (root_parent.bind(), std::convert::identity as fn(u64) -> u64).into();
            for infile in create_cmd.infiles {
                creator.push_back(Entry::new(infile, root_parent.clone())?);
            }

            creator.run(create_cmd.outfile)
        }

        Commands::List(list_cmd) => {
            if args.verbose > 0 {
                println!("Listing entries in archive {:?}", list_cmd.infile);
            }

            arx::list(list_cmd.infile)
        }

        Commands::Dump(dump_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Dump entry {} in archive {:?}",
                    dump_cmd.path, dump_cmd.infile
                );
            }

            arx::dump(dump_cmd.infile, dump_cmd.path.into())
        }

        Commands::Extract(extract_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Extract archive {:?} in {:?}",
                    extract_cmd.infile, extract_cmd.outdir
                );
            }

            arx::extract(extract_cmd.infile, extract_cmd.outdir)
        }

        Commands::Mount(mount_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Mount archive {:?} in {:?}",
                    mount_cmd.infile, mount_cmd.mountdir
                );
            }

            arx::mount(mount_cmd.infile, mount_cmd.mountdir)
        }
    }
}
