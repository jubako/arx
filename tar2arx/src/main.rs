use bstr::{BString, ByteVec};
use clap::{CommandFactory, Parser, ValueHint};

use anyhow::{anyhow, Result};
use jbk::creator::ContentAdder;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;

const VERSION: &str = const_format::formatcp!(
    "{}{}",
    clap::crate_version!(),
    git_version::git_version!(
        args = ["--dirty=*", "--tags", "--always"],
        fallback = "",
        prefix = " (git:",
        suffix = ")"
    )
);

/// Convert a tar archive into an Arx archive.
///
/// The tar content (uncompressed) must be passed to stdin.
#[derive(Parser)]
#[command(name = "tar2arx", author, version, long_version=VERSION, about, long_about=None)]
struct Cli {
    /// Tar file to convert
    #[arg(value_parser, value_hint=ValueHint::FilePath)]
    tar_file: Option<String>,

    /// Archive name to create
    #[arg(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath
    )]
    outfile: Option<jbk::Utf8PathBuf>,

    #[command(flatten)]
    concat_mode: Option<jbk::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(
        short,
        long,
        value_parser=jbk::cmd_utils::compression_arg_parser,
        required=false,
        default_value = "zstd"
    )]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,

    #[arg(long, help_heading = "Advanced")]
    generate_man_page: bool,

    #[arg(long, help_heading = "Advanced")]
    generate_complete: Option<clap_complete::Shell>,
}

#[derive(Clone)]
struct ProgressBar {
    pub comp_clusters: indicatif::ProgressBar,
    pub uncomp_clusters: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new(size_progress_bar: indicatif::ProgressBar) -> Self {
        let draw_target = indicatif::ProgressDrawTarget::stdout_with_hz(1);
        let style = indicatif::ProgressStyle::with_template(
            "{prefix} : [{wide_bar:.cyan/blue}] {pos:7} / {len:7}",
        )
        .unwrap()
        .progress_chars("#+- ");

        let multi = indicatif::MultiProgress::with_draw_target(draw_target);

        let comp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style.clone())
            .with_prefix("Compressed Cluster  ");

        let uncomp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style.clone())
            .with_prefix("Uncompressed Cluster");

        let bytes_style = style
            .template(
                "{elapsed} / {duration} : [{wide_bar:.cyan/blue}] {bytes:7} / {total_bytes:7}",
            )
            .unwrap();

        let size = size_progress_bar
            .with_style(bytes_style)
            .with_prefix("Size");

        multi.add(size.clone());
        multi.add(comp_clusters.clone());
        multi.add(uncomp_clusters.clone());
        Self {
            comp_clusters,
            uncomp_clusters,
        }
    }
}

impl jbk::creator::Progress for ProgressBar {
    fn new_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc_length(1)
    }
    fn handle_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc(1)
    }
    fn content_added(&self, _size: jbk::Size) {}
}

pub struct Converter<R: Read> {
    arx_creator: arx::create::SimpleCreator,
    archive: tar::Archive<R>,
}

struct TarEntry {
    path: arx::PathBuf,
    kind: arx::create::EntryKind,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl TarEntry {
    pub fn new<'a, R: 'a + Read>(
        mut entry: tar::Entry<'a, R>,
        adder: &mut impl ContentAdder,
    ) -> Result<Option<Self>, arx::CreatorError> {
        let header = entry.header();
        let uid = header.uid()?;
        let gid = header.gid()?;
        let mtime = header.mtime()?;
        let mode = header.mode()? as u64;
        let path = arx::PathBuf::from_path(entry.path()?)
            .unwrap_or_else(|_| panic!("Entry path must be utf-8"));
        Ok(match header.entry_type() {
            tar::EntryType::Directory => Some(Self {
                path,
                kind: arx::create::EntryKind::Dir,
                uid,
                gid,
                mtime,
                mode,
            }),
            tar::EntryType::Symlink => {
                let target = entry.link_name()?.unwrap();
                Some(Self {
                    path,
                    kind: arx::create::EntryKind::Link(BString::new(
                        Vec::from_path_buf(target.into_owned())
                            .unwrap_or_else(|target| panic!("{target:?} must be utf8")),
                    )),
                    uid,
                    gid,
                    mtime,
                    mode,
                })
            }
            /* GNULongName, GNULongLink and XHeader should already be handled by entries iterator
               but it doesn't arm to explicitly ignore them.
               XGlobalHeader is not handled by entries iterator, so we MUST explicitly ignore it.
            */
            tar::EntryType::GNULongName
            | tar::EntryType::GNULongLink
            | tar::EntryType::XHeader
            | tar::EntryType::XGlobalHeader => None,
            _ => {
                if header.as_ustar().is_none() && header.path_bytes().ends_with(b"/") {
                    Some(Self {
                        path,
                        kind: arx::create::EntryKind::Dir,
                        uid,
                        gid,
                        mtime,
                        mode,
                    })
                } else {
                    //Handle everything else as normal file
                    let mut data = vec![];
                    let size = entry.read_to_end(&mut data)?;
                    let content_address = adder.add_content(
                        Box::new(std::io::Cursor::new(data)),
                        jbk::creator::CompHint::Detect,
                    )?;
                    Some(Self {
                        path,
                        kind: arx::create::EntryKind::File(size.into(), content_address),
                        uid,
                        gid,
                        mtime,
                        mode,
                    })
                }
            }
        })
    }
}

impl arx::create::EntryTrait for TarEntry {
    fn kind(&self) -> Result<Option<arx::create::EntryKind>, arx::CreatorError> {
        Ok(Some(self.kind.clone()))
    }
    fn path(&self) -> &arx::Path {
        &self.path
    }

    fn uid(&self) -> u64 {
        self.uid
    }
    fn gid(&self) -> u64 {
        self.gid
    }
    fn mode(&self) -> u64 {
        self.mode
    }
    fn mtime(&self) -> u64 {
        self.mtime
    }
}

impl<R: Read> Converter<R> {
    pub fn new(
        archive: tar::Archive<R>,
        outfile: impl AsRef<jbk::Utf8Path>,
        concat_mode: jbk::creator::ConcatMode,
        compression: jbk::creator::Compression,
        progress_bar: indicatif::ProgressBar,
    ) -> Result<Self, arx::CreatorError> {
        let progress = Arc::new(ProgressBar::new(progress_bar));
        let arx_creator = arx::create::SimpleCreator::new(
            outfile,
            concat_mode,
            progress,
            Rc::new(()),
            compression,
        )?;

        Ok(Self {
            arx_creator,
            archive,
        })
    }

    fn finalize(self) -> Result<(), arx::CreatorError> {
        self.arx_creator.finalize()
    }

    pub fn run(mut self) -> Result<(), arx::CreatorError> {
        let iter = self.archive.entries()?;
        for entry in iter {
            let entry = entry?;
            if let Some(entry) = TarEntry::new(entry, self.arx_creator.adder())? {
                self.arx_creator.add_entry(&entry)?;
            }
        }
        self.finalize()
    }
}

fn main() -> Result<()> {
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .homepage(env!("CARGO_PKG_HOMEPAGE")));

    let args = Cli::parse();

    if args.list_compressions {
        jbk::cmd_utils::list_compressions();
        return Ok(());
    }

    if args.generate_man_page {
        let man = clap_mangen::Man::new(Cli::command());
        man.render(&mut std::io::stdout())?;
        return Ok(());
    }

    if let Some(what) = args.generate_complete {
        let mut command = Cli::command();
        let name = command.get_name().to_string();
        clap_complete::generate(what, &mut command, name, &mut std::io::stdout());
        return Ok(());
    }

    let mut input_size = None;

    if args.outfile.is_none() && (args.tar_file.is_none() || args.tar_file.as_ref().unwrap() == "-")
    {
        return Err(anyhow!("Cannot infer arx name from input name. Please give a output filename with `--outfile` option."));
    }

    let outfile = args.outfile.unwrap_or_else(|| {
        let p = jbk::Utf8Path::new(args.tar_file.as_ref().unwrap());
        let p = if p.starts_with("https://") || p.starts_with("http://") {
            jbk::Utf8Path::new(p.file_name().unwrap())
                .with_extension("")
                .with_extension("arx")
        } else {
            p.with_extension("").with_extension("arx")
        };
        println!("Converting tar file to {p:?}");
        p
    });

    let input: Box<dyn Read> = match args.tar_file {
        None => Box::new(std::io::stdin()),
        Some(p) => {
            if p == "-" {
                Box::new(std::io::stdin())
            } else if p.starts_with("https://") || p.starts_with("http://") {
                #[cfg(feature = "http")]
                {
                    Box::new(ureq::get(&p).call()?.into_reader())
                }
                #[cfg(not(feature = "http"))]
                return Err(anyhow!("Cannot open remote tar without http feature"));
            } else {
                let f = std::fs::File::open(p)?;
                input_size = Some(f.metadata()?.len());
                Box::new(f)
            }
        }
    };
    let progress_bar = indicatif::ProgressBar::hidden();
    if let Some(s) = input_size {
        progress_bar.set_length(s);
    };
    let input_stream = niffler::get_reader(Box::new(progress_bar.wrap_read(input)))
        .unwrap()
        .0;

    let archive = tar::Archive::new(input_stream);
    let converter = Converter::new(
        archive,
        &outfile,
        match args.concat_mode {
            None => jbk::creator::ConcatMode::OneFile,
            Some(e) => e.into(),
        },
        args.compression,
        progress_bar,
    )?;
    Ok(converter.run()?)
}
