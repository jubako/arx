use clap::{Parser, ValueHint};
use daemonize::Daemonize;
use log::info;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct StatCounter {
    nb_lookup: u64,
    nb_getattr: u64,
    nb_readlink: u64,
    nb_open: u64,
    nb_read: u64,
    nb_release: u64,
    nb_opendir: u64,
    nb_readdir: u64,
    nb_releasedir: u64,
}

impl StatCounter {
    pub fn new() -> Self {
        Self {
            nb_lookup: 0,
            nb_getattr: 0,
            nb_readlink: 0,
            nb_open: 0,
            nb_read: 0,
            nb_release: 0,
            nb_opendir: 0,
            nb_readdir: 0,
            nb_releasedir: 0,
        }
    }
}

impl arx::Stats for StatCounter {
    fn lookup(&mut self) {
        self.nb_lookup += 1;
    }

    fn getattr(&mut self) {
        self.nb_getattr += 1;
    }

    fn readlink(&mut self) {
        self.nb_readlink += 1;
    }

    fn open(&mut self) {
        self.nb_open += 1;
    }

    fn read(&mut self) {
        self.nb_read += 1;
    }

    fn release(&mut self) {
        self.nb_release += 1;
    }

    fn opendir(&mut self) {
        self.nb_opendir += 1;
    }

    fn readdir(&mut self) {
        self.nb_readdir += 1;
    }

    fn releasedir(&mut self) {
        self.nb_releasedir += 1;
    }
}

impl Default for StatCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StatCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "nb_lookup: {}", self.nb_lookup)?;
        writeln!(f, "nb_getattr: {}", self.nb_getattr)?;
        writeln!(f, "nb_readlink: {}", self.nb_readlink)?;
        writeln!(f, "nb_open: {}", self.nb_open)?;
        writeln!(f, "nb_read: {}", self.nb_read)?;
        writeln!(f, "nb_release: {}", self.nb_release)?;
        writeln!(f, "nb_opendir: {}", self.nb_opendir)?;
        writeln!(f, "nb_readdir: {}", self.nb_readdir)?;
        writeln!(f, "nb_releasedir: {}", self.nb_releasedir)?;
        Ok(())
    }
}

/// Mount an archive in a directory.
#[derive(Parser, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_parser, value_hint=ValueHint::FilePath)]
    infile: PathBuf,

    /// Target directory
    #[arg(value_parser, value_hint=ValueHint::DirPath)]
    mountdir: Option<PathBuf>,

    /// Root directory
    #[arg(long)]
    root_dir: Option<PathBuf>,

    /// Share mount point with other users
    #[arg(long)]
    allow_other: bool,

    /// Share mount point with root user
    #[arg(long)]
    allow_root: bool,

    /// Forground operation
    #[arg(short, long)]
    foreground: bool,

    #[arg(from_global)]
    verbose: u8,
}

type DummyBuilder = ((), (), ());

pub fn mount(options: Options) -> anyhow::Result<()> {
    let mut stats = StatCounter::new();
    let arx = arx::Arx::new(&options.infile)?;

    let root_range = match options.root_dir {
        None => (&arx.root_index).into(),
        Some(p) => {
            let relative_path = arx::Path::from_path(&p)?;
            let root = arx.get_entry::<DummyBuilder>(relative_path)?;
            match root {
                arx::Entry::Dir(range, _) => range,
                _ => return Err(anyhow::anyhow!("{} must be a directory", p.display())),
            }
        }
    };

    let arxfs = arx::ArxFs::new_with_stats(arx, root_range, &mut stats)?;

    let mut abs_path = std::env::current_dir().unwrap();
    abs_path = abs_path.join(options.infile);
    let mut _tmp = None;
    let mount_dir = match &options.mountdir {
        Some(m) => m.as_path(),
        None => {
            let file_name = abs_path.file_name().unwrap();
            let mut prefix = OsString::with_capacity(file_name.len() + 1);
            prefix.push(file_name);
            prefix.push(".");
            _tmp = Some(tempfile::TempDir::with_prefix_in(
                prefix,
                abs_path.parent().unwrap(),
            )?);
            println!(
                "Create mount point {}",
                _tmp.as_ref().unwrap().path().display()
            );
            _tmp.as_ref().unwrap().path()
        }
    };

    let mut abs_mount_point = std::env::current_dir().unwrap();
    abs_mount_point = abs_mount_point.join(mount_dir);

    if !options.foreground {
        let daemonize = Daemonize::new()
            .stdout(daemonize::Stdio::keep())
            .stderr(daemonize::Stdio::keep());
        if let Err(e) = daemonize.start() {
            eprintln!("Error daemonize, {}", e);
            return Err(anyhow::anyhow!("Failed to daemonize."));
        }
    }
    info!(
        "Mount {} in {}",
        abs_path.display(),
        abs_mount_point.display()
    );
    arxfs.mount(
        abs_path.to_str().unwrap().to_string(),
        abs_mount_point,
        options.allow_other,
        options.allow_root,
    )?;

    info!("Stats:\n {stats}");
    Ok(())
}
