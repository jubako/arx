use clap::{Parser, ValueHint};
use log::info;
use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Extract the content of an archive
#[derive(Parser, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_hint=ValueHint::FilePath, required_unless_present("infile_old"))]
    infile: Option<PathBuf>,

    /// Directory in which extract the archive. (Default to current directory)
    #[arg(short = 'C', required = false, value_hint=ValueHint::DirPath)]
    outdir: Option<PathBuf>,

    /// Files to extract
    #[arg(group = "input", value_hint=ValueHint::AnyPath)]
    extract_files: Vec<arx::PathBuf>,

    /// Root directory
    #[arg(long)]
    root_dir: Option<PathBuf>,

    /// Print a progress bar of the extraction
    #[arg(short = 'p', long = "progress", default_value_t = false, action)]
    progress: bool,

    /// Get the list of files/directories to extract from the FILE_LIST (incompatible with EXTRACT_FILES)
    #[arg(
        short = 'L',
        long = "file-list",
        group = "input",
        value_hint = ValueHint::FilePath
    )]
    file_list: Option<PathBuf>,

    #[arg(from_global)]
    verbose: u8,

    /// Recursively extract directories
    ///
    /// Default value is true if `EXTRACT_FILES` is passed and false is `FILE_LIST` is passed.
    #[arg(
        short,
        long,
        required = false,
        default_value_t = false,
        default_value_ifs([
            ("no_recurse", clap::builder::ArgPredicate::IsPresent, "false"),
            ("extract_files", clap::builder::ArgPredicate::IsPresent, "true")
        ]),
        conflicts_with = "no_recurse",
        action
    )]
    recurse: bool,

    /// Force `--recurse` to be false.
    #[arg(long)]
    no_recurse: bool,

    #[arg(
        short = 'f',
        long = "file",
        hide = true,
        conflicts_with("infile"),
        required_unless_present("infile")
    )]
    infile_old: Option<PathBuf>,

    #[arg(long, default_value = "warn")]
    overwrite: arx::Overwrite,
}

fn get_files_to_extract(options: &Options) -> jbk::Result<HashSet<arx::PathBuf>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list)?;
        let mut files: HashSet<arx::PathBuf> = Default::default();
        for line in BufReader::new(file).lines() {
            files.insert(line?.into());
        }
        Ok(files)
    } else {
        Ok(options.extract_files.iter().cloned().collect())
    }
}

type DummyBuilder = ((), (), ());

pub fn extract(options: Options) -> anyhow::Result<()> {
    let files_to_extract = get_files_to_extract(&options)?;
    let outdir = match options.outdir {
        Some(o) => o,
        None => current_dir()?,
    };
    let infile = if let Some(ref infile) = options.infile_old {
        infile
    } else {
        options.infile.as_ref().unwrap()
    };
    let arx = arx::Arx::new(infile)?;
    info!("Extract archive {:?} in {:?}", &infile, outdir);

    match options.root_dir {
        None => arx::extract_arx(
            &arx,
            &outdir,
            files_to_extract,
            options.recurse,
            options.progress,
            options.overwrite,
        )?,
        Some(p) => {
            let relative_path = arx::Path::from_path(&p)?;
            let root = arx.get_entry::<DummyBuilder>(relative_path)?;
            match root {
                arx::Entry::Dir(range, _) => arx::extract_arx_range(
                    &arx,
                    &outdir,
                    &range,
                    files_to_extract,
                    options.recurse,
                    options.progress,
                    options.overwrite,
                )?,
                _ => return Err(anyhow::anyhow!("{} must be a directory", p.display())),
            }
        }
    };
    Ok(())
}
