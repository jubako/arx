use clap::Parser;
use log::error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mount.fuse.arx", author, version, about, long_about=None)]
struct Cli {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser)]
    mountdir: PathBuf,

    #[arg(short)]
    option: Vec<String>,
}

fn mount<INP, OUTP>(infile: INP, outdir: OUTP) -> jbk::Result<()>
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

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    if args.option.contains(&"rw".into()) {
        error!("arx cannot be mounted rw");
        return Err("arx cannot be mounted rw".into());
    }

    mount(args.infile, args.mountdir)
}
