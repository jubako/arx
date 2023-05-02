use jubako as jbk;

mod create;
mod dump;
mod extract;
mod light_path;
mod list;
mod mount;

use clap::{Parser, Subcommand};

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
    Create(create::Options),

    #[clap(arg_required_else_help = true)]
    List(list::Options),

    #[clap(arg_required_else_help = true)]
    Dump(dump::Options),

    #[clap(arg_required_else_help = true)]
    Extract(extract::Options),

    #[clap(arg_required_else_help = true)]
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

fn main() -> jbk::Result<()> {
    configure_log();
    let args = Cli::parse();

    match args.command {
        Commands::Create(options) => create::create(options, args.verbose),
        Commands::List(options) => list::list(options, args.verbose),
        Commands::Dump(options) => dump::dump(options, args.verbose),
        Commands::Extract(options) => extract::extract(options, args.verbose),
        Commands::Mount(options) => mount::mount(options, args.verbose),
    }
}
