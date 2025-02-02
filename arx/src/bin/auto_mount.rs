use std::process::ExitCode;

#[cfg(unix)]
mod inner {
    pub use clap::Parser;
    pub use std::env;
    use std::path::PathBuf;

    #[derive(Parser)]
    #[command(name = "arx", author, version, about, long_about=None)]
    pub struct Cli {
        #[arg(short, long, action=clap::ArgAction::Count)]
        pub verbose: u8,

        #[arg(value_parser)]
        pub mountdir: PathBuf,
    }

    pub fn mount<INP, OUTP>(infile: INP, outdir: OUTP) -> Result<(), arx::ArxError>
    where
        INP: AsRef<std::path::Path>,
        OUTP: AsRef<std::path::Path>,
    {
        let arx = arx::Arx::new(&infile)?;
        let arxfs = arx::ArxFs::new(arx)?;

        let mut abs_path = env::current_dir().unwrap();
        abs_path = abs_path.join(infile.as_ref());

        arxfs.mount(abs_path.to_str().unwrap().to_string(), &outdir)
    }
}

#[cfg(unix)]
fn main() -> ExitCode {
    use inner::*;

    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .homepage(env!("CARGO_PKG_HOMEPAGE")));
    let args = Cli::parse();

    match env::current_exe() {
        Ok(exe_path) => {
            if args.verbose > 0 {
                print!("Auto Mount archive {:?} in {:?}", exe_path, args.mountdir);
            }
            match mount(exe_path, args.mountdir) {
                Ok(()) => ExitCode::SUCCESS,
                Err(arx::ArxError::BaseError(arx::BaseError::Jbk(_))) => {
                    eprintln!("Impossible to locate a Jubako archive in the executable.");
                    eprintln!("This binary is not intented to be directly used, you must put a Jubako archive at its end.");
                    ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("failed to get current exe path: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn main() -> ExitCode {
    eprintln!("Mount feature is not available on Windows.");
    ExitCode::FAILURE
}
