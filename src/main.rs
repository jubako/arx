use jubako as jbk;
use libarx as arx;

mod dump;
mod extract;
mod light_path;
mod list;
mod mount;

use clap::{Args, Parser, Subcommand};
use std::env::current_dir;
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

    #[clap(long = "stable-output", action)]
    stable_output: Option<u8>,
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
    #[clap(short = 'f', long = "file")]
    infile: PathBuf,

    #[clap(short = 'C', required = false)]
    outdir: Option<PathBuf>,

    #[clap(value_parser)]
    extract_files: Vec<PathBuf>,
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

            let mut creator = arx::create::Creator::new(&create_cmd.outfile)?;

            for infile in create_cmd.infiles {
                creator.add_from_path(infile)?;
            }

            creator.finalize(create_cmd.outfile)
        }

        Commands::List(list_cmd) => {
            if args.verbose > 0 {
                println!("Listing entries in archive {:?}", list_cmd.infile);
            }

            list::list(list_cmd.infile, list_cmd.stable_output)
        }

        Commands::Dump(dump_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Dump entry {} in archive {:?}",
                    dump_cmd.path, dump_cmd.infile
                );
            }

            dump::dump(dump_cmd.infile, dump_cmd.path.into())
        }

        Commands::Extract(extract_cmd) => {
            let outdir = match extract_cmd.outdir {
                Some(o) => o,
                None => current_dir()?,
            };
            if args.verbose > 0 {
                println!("Extract archive {:?} in {:?}", extract_cmd.infile, outdir);
            }

            extract::extract(extract_cmd.infile, outdir, extract_cmd.extract_files)
        }

        Commands::Mount(mount_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Mount archive {:?} in {:?}",
                    mount_cmd.infile, mount_cmd.mountdir
                );
            }

            mount::mount(mount_cmd.infile, mount_cmd.mountdir)
        }
    }
}
