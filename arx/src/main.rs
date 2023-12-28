mod create;
mod dump;
mod extract;
mod light_path;
mod list;
#[cfg(not(windows))]
mod mount;

use anyhow::Result;
use clap::Parser;
use log::error;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "arx", author, version, about, long_about=None)]
struct Cli {
    /// Set verbose level. Can be specify several times to augment verbose level.
    #[arg(short, long, action=clap::ArgAction::Count, global=true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
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

    #[cfg(not(windows))]
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

    match args.command {
        Commands::Create(options) => create::create(options),
        Commands::List(options) => Ok(list::list(options)?),
        Commands::Dump(options) => Ok(dump::dump(options)?),
        Commands::Extract(options) => Ok(extract::extract(options)?),
        #[cfg(not(windows))]
        Commands::Mount(options) => Ok(mount::mount(options)?),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Error : {e:#}");
            ExitCode::FAILURE
        }
    }
}
