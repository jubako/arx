#[cfg(unix)]
mod inner {
    pub use clap::Parser;
    use std::path::PathBuf;
    #[derive(Parser)]
    #[command(name = "mount.fuse.arx", author, version, about, long_about=None)]
    pub struct Cli {
        #[arg(value_parser)]
        pub infile: PathBuf,

        #[arg(value_parser)]
        pub mountdir: PathBuf,

        #[arg(short)]
        pub option: Vec<String>,
    }

    pub fn mount<INP, OUTP>(infile: INP, outdir: OUTP) -> Result<(), arx::ArxError>
    where
        INP: AsRef<std::path::Path>,
        OUTP: AsRef<std::path::Path>,
    {
        let arx = arx::Arx::new(&infile)?;
        let arxfs = arx::ArxFs::new(arx)?;

        let mut abs_path = std::env::current_dir().unwrap();
        abs_path = abs_path.join(infile.as_ref());

        arxfs.mount(abs_path.to_str().unwrap().to_string(), &outdir)
    }
}

#[cfg(unix)]
fn main() -> Result<(), arx::MountError> {
    use inner::*;

    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .homepage(env!("CARGO_PKG_HOMEPAGE")));
    let args = Cli::parse();

    if args.option.contains(&"rw".into()) {
        eprintln!("arx cannot be mounted rw");
        return Err(arx::MountError::CannotMountRW);
    }

    Ok(mount(args.infile, args.mountdir)?)
}

#[cfg(windows)]
fn main() -> jbk::Result<()> {
    Err("Mount feature is not availble on Windows.".into())
}
