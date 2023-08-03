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

impl libarx::Stats for StatCounter {
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

#[derive(clap::Args)]
pub struct Options {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    mountdir: PathBuf,
}

pub fn mount(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!(
            "Mount archive {:?} in {:?}",
            options.infile, options.mountdir
        );
    }
    let mut stats = StatCounter::new();
    let arx = libarx::Arx::new(options.infile)?;
    let arxfs = libarx::ArxFs::new_with_stats(arx, &mut stats)?;

    arxfs.mount(&options.mountdir)?;

    println!("Stats:\n {stats}");
    Ok(())
}
