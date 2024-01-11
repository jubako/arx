use clap::Parser;

use arx::create::Adder;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// Convert a tar archive into an Arx archive.
///
/// The tar content (uncompressed) must be passed to stdin.
#[derive(Parser)]
#[command(name = "tar2arx", author, version, about, long_about=None)]
struct Cli {
    /// Archive name to create
    #[arg(
        short,
        long,
        value_parser,
        required_unless_present("list_compressions")
    )]
    outfile: Option<PathBuf>,

    #[command(flatten)]
    concat_mode: Option<arx::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(short, long, value_parser=arx::cmd_utils::compression_arg_parser, required=false, default_value = "zstd")]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,
}

#[derive(Clone)]
struct ProgressBar {
    pub comp_clusters: indicatif::ProgressBar,
    pub uncomp_clusters: indicatif::ProgressBar,
    pub size: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new() -> jbk::Result<Self> {
        Self::new_with_size(None)
    }

    fn new_with_size(size: Option<u64>) -> jbk::Result<Self> {
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

        let size = match size {
            None => indicatif::ProgressBar::new_spinner(),
            Some(s) => indicatif::ProgressBar::new(s),
        }
        .with_style(bytes_style)
        .with_prefix("Size");

        multi.add(size.clone());
        multi.add(comp_clusters.clone());
        multi.add(uncomp_clusters.clone());
        Ok(Self {
            comp_clusters,
            uncomp_clusters,
            size,
        })
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
    fn content_added(&self, size: jbk::Size) {
        self.size.inc(size.into_u64())
    }
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
    pub fn new<'a, R: 'a + Read, A: Adder>(
        mut entry: tar::Entry<'a, R>,
        adder: &mut A,
    ) -> jbk::Result<Option<Self>> {
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
                    kind: arx::create::EntryKind::Link(
                        arx::PathBuf::from_path(&target)
                            .unwrap_or_else(|_| panic!("{target:?} must be utf8")),
                    ),
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
                    let content_address = adder.add(std::io::Cursor::new(data))?;
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
    fn kind(&self) -> jbk::Result<Option<arx::create::EntryKind>> {
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
    pub fn new<P: AsRef<Path>>(
        archive: tar::Archive<R>,
        outfile: P,
        concat_mode: arx::create::ConcatMode,
    ) -> jbk::Result<Self> {
        let progress = Arc::new(ProgressBar::new()?);
        let arx_creator = arx::create::SimpleCreator::new(
            outfile,
            concat_mode,
            progress,
            Rc::new(()),
            jbk::creator::Compression::zstd(),
        )?;

        Ok(Self {
            arx_creator,
            archive,
        })
    }

    fn finalize(self, outfile: &Path) -> jbk::Result<()> {
        self.arx_creator.finalize(outfile)
    }

    pub fn run(mut self, outfile: &Path) -> jbk::Result<()> {
        let iter = self.archive.entries()?;
        for entry in iter {
            let entry = entry?;
            if let Some(entry) = TarEntry::new(entry, self.arx_creator.adder())? {
                self.arx_creator.add_entry(&entry)?;
            }
        }
        self.finalize(outfile)
    }
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    if args.list_compressions {
        arx::cmd_utils::list_compressions();
        return Ok(());
    }

    let stdin = std::io::stdin();
    let archive = tar::Archive::new(stdin.lock());
    let converter = Converter::new(
        archive,
        args.outfile.as_ref().unwrap(),
        args.concat_mode.into(),
    )?;
    converter.run(args.outfile.as_ref().unwrap())
}
