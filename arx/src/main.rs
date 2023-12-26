mod create;
mod dump;
mod extract;
mod light_path;
mod list;
mod mount;

use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "arx", author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Create(create::Options),

    #[command(arg_required_else_help = true)]
    List(list::Options),

    #[command(arg_required_else_help = true)]
    Dump(dump::Options),

    #[command(arg_required_else_help = true)]
    Extract(extract::Options),

    #[command(arg_required_else_help = true)]
    Mount(mount::Options),
}

fn configure_log() {
    let env = env_logger::Env::default()
        .filter("ARX_LOG")
        .write_style("ARX_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_module_path(false)
        .format_timestamp(None)
        .init();
}

fn run() -> Result<()> {
    configure_log();
    let args = Cli::parse();

    match args.command {
        Commands::Create(options) => create::create(options, args.verbose),
        Commands::List(options) => Ok(list::list(options, args.verbose)?),
        Commands::Dump(options) => Ok(dump::dump(options, args.verbose)?),
        Commands::Extract(options) => Ok(extract::extract(options, args.verbose)?),
        Commands::Mount(options) => Ok(mount::mount(options, args.verbose)?),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error : {e:#}");
            ExitCode::FAILURE
        }
    }
}
