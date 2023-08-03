use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[clap(name = "arx")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    #[clap(value_parser)]
    mountdir: PathBuf,
}

fn mount<INP, OUTP>(infile: INP, outdir: OUTP) -> jbk::Result<()>
where
    INP: AsRef<std::path::Path>,
    OUTP: AsRef<std::path::Path>,
{
    let arx = arx::Arx::new(infile)?;
    let arxfs = arx::ArxFs::new(arx)?;

    arxfs.mount(&outdir)
}

fn main() -> ExitCode {
    let args = Cli::parse();

    match env::current_exe() {
        Ok(exe_path) => {
            if args.verbose > 0 {
                println!("Auto Mount archive {:?} in {:?}", exe_path, args.mountdir);
            }
            match mount(exe_path, args.mountdir) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => match e.error {
                    jbk::ErrorKind::NotAJbk => {
                        eprintln!("Impossible to locate a Jubako archive in the executable.");
                        eprintln!("This binary is not intented to be directly used, you must put a Jubako archive at its end.");
                        ExitCode::FAILURE
                    }
                    _ => {
                        eprintln!("Error: {e}");
                        ExitCode::FAILURE
                    }
                },
            }
        }
        Err(e) => {
            eprintln!("failed to get current exe path: {e}");
            ExitCode::FAILURE
        }
    }
}
