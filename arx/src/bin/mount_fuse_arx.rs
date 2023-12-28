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

    pub fn mount<INP, OUTP>(infile: INP, outdir: OUTP) -> jbk::Result<()>
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
fn main() -> jbk::Result<()> {
    use inner::*;
    use log::error;
    let args = Cli::parse();

    if args.option.contains(&"rw".into()) {
        error!("arx cannot be mounted rw");
        return Err("arx cannot be mounted rw".into());
    }

    mount(args.infile, args.mountdir)
}

#[cfg(windows)]
fn main() -> jbk::Result<()> {
    Err("Mount feature is not availble on Windows.".into())
}
