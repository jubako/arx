mod create;
mod dump;
mod extract;
mod light_path;
mod list;
#[cfg(all(not(windows), feature = "fuse"))]
mod mount;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use log::error;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "arx", author, version, about, long_about=None)]
struct Cli {
    /// Set verbose level. Can be specify several times to augment verbose level.
    #[arg(short, long, action=clap::ArgAction::Count, global=true)]
    verbose: u8,

    #[arg(
        long,
        num_args= 0..=1,
        default_missing_value = "",
        help_heading = "Advanced",
        value_parser([
            "",
            "create",
            "list",
            "dump",
            "extract",
            #[cfg(all(not(windows), feature = "fuse"))]
            "mount"
        ])
    )]
    generate_man_page: Option<String>,

    #[arg(long, help_heading = "Advanced")]
    generate_complete: Option<clap_complete::Shell>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Create(create::Options),

    #[command(arg_required_else_help = true)]
    List(list::Options),

    #[command(arg_required_else_help = true)]
    Dump(dump::Options),

    #[command(arg_required_else_help = true)]
    Extract(extract::Options),

    #[cfg(all(not(windows), feature = "fuse"))]
    #[command(arg_required_else_help = true)]
    Mount(mount::Options),
}

fn configure_log(verbose: u8) {
    let env = env_logger::Env::default()
        .filter("ARX_LOG")
        .write_style("ARX_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .filter_module(
            "arx",
            match verbose {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
        )
        .format_module_path(false)
        .format_timestamp(None)
        .init();
}

fn run() -> Result<()> {
    let args = Cli::parse();
    configure_log(args.verbose);

    if let Some(what) = args.generate_man_page {
        let command = match what.as_str() {
            "" => Cli::command(),
            "create" => create::Options::command(),
            "list" => list::Options::command(),
            "dump" => dump::Options::command(),
            "extract" => extract::Options::command(),
            #[cfg(all(not(windows), feature = "fuse"))]
            "mount" => mount::Options::command(),
            _ => return Ok(Cli::command().print_help()?),
        };
        let man = clap_mangen::Man::new(command);
        man.render(&mut std::io::stdout())?;
        return Ok(());
    }

    if let Some(what) = args.generate_complete {
        let mut command = Cli::command();
        let name = command.get_name().to_string();
        clap_complete::generate(what, &mut command, name, &mut std::io::stdout());
        return Ok(());
    }

    match args.command {
        None => Ok(Cli::command().print_help()?),
        Some(c) => match c {
            Commands::Create(options) => create::create(options),
            Commands::List(options) => Ok(list::list(options)?),
            Commands::Dump(options) => Ok(dump::dump(options)?),
            Commands::Extract(options) => Ok(extract::extract(options)?),
            #[cfg(all(not(windows), feature = "fuse"))]
            Commands::Mount(options) => Ok(mount::mount(options)?),
        },
    }
}

fn main() -> ExitCode {
    human_panic::setup_panic!();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Error : {e:#}");
            ExitCode::FAILURE
        }
    }
}
