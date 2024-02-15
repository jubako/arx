use clap::{CommandFactory, Parser};

use arx::create::Adder;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// Convert a zip archive into an Arx archive.
#[derive(Parser)]
#[command(name = "tar2arx", author, version, about, long_about=None)]
struct Cli {
    /// Zip file to convert
    #[arg(
        value_parser,
        required_unless_present_any(["list_compressions", "generate_man_page"])
    )]
    zip_file: Option<PathBuf>,

    #[arg(long, help_heading = "Advanced")]
    generate_man_page: bool,

    /// Archive name to create
    #[arg(
        short,
        long,
        value_parser,
        required_unless_present_any(["list_compressions", "generate_man_page"])
    )]
    outfile: Option<PathBuf>,

    #[command(flatten)]
    concat_mode: Option<arx::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(
        short,
        long,
        value_parser=arx::cmd_utils::compression_arg_parser,
        required=false,
        default_value = "zstd"
    )]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,
}

#[derive(Clone)]
struct ProgressBar {
    pub comp_clusters: indicatif::ProgressBar,
    pub uncomp_clusters: indicatif::ProgressBar,
    pub entries: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new<R: Read + Seek>(archive: &zip::ZipArchive<R>) -> jbk::Result<Self> {
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

        let entries_style = style
            .template("{elapsed} / {duration} : [{wide_bar:.cyan/blue}] {pos:7} / {len:7}")
            .unwrap();
        let entries = indicatif::ProgressBar::new(archive.len() as u64).with_style(entries_style);

        multi.add(entries.clone());
        multi.add(comp_clusters.clone());
        multi.add(uncomp_clusters.clone());
        Ok(Self {
            comp_clusters,
            uncomp_clusters,
            entries,
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
}

pub struct Converter<R: Read + Seek> {
    arx_creator: arx::create::SimpleCreator,
    archive_path: PathBuf,
    archive: zip::ZipArchive<R>,
    progress: Arc<ProgressBar>,
}

struct ZipEntry {
    path: arx::PathBuf,
    kind: arx::create::EntryKind,
    mode: u64,
    mtime: u64,
}

impl ZipEntry {
    pub fn new<A: Adder>(
        mut entry: zip::read::ZipFile<'_>,
        adder: &mut A,
        archive_path: &Path,
    ) -> jbk::Result<Self> {
        let mtime = entry.last_modified().to_time().unwrap().unix_timestamp() as u64;
        let mode = entry.unix_mode().unwrap_or(0o644) as u64;
        let path = entry.enclosed_name();
        if path.is_none() {
            return Err("Invalid path".into());
        }
        let path = arx::PathBuf::from_path(path.unwrap())
            .unwrap_or_else(|_| panic!("{path:?} must be utf8"));

        Ok(if entry.is_dir() {
            Self {
                path,
                kind: arx::create::EntryKind::Dir,
                mtime,
                mode,
            }
        } else {
            let content_address = if let zip::CompressionMethod::Stored = entry.compression() {
                let reader = jbk::creator::InputFile::new_range(
                    std::fs::File::open(archive_path)?,
                    entry.data_start(),
                    Some(entry.size()),
                )?;
                adder.add(reader)?
            } else {
                let mut data = vec![];
                entry.read_to_end(&mut data)?;
                adder.add(std::io::Cursor::new(data))?
            };
            Self {
                path,
                kind: arx::create::EntryKind::File(entry.size().into(), content_address),
                mtime,
                mode,
            }
        })
    }
}

impl arx::create::EntryTrait for ZipEntry {
    fn kind(&self) -> jbk::Result<Option<arx::create::EntryKind>> {
        Ok(Some(self.kind.clone()))
    }
    fn path(&self) -> &arx::Path {
        &self.path
    }

    fn uid(&self) -> u64 {
        0
    }
    fn gid(&self) -> u64 {
        0
    }
    fn mode(&self) -> u64 {
        self.mode
    }
    fn mtime(&self) -> u64 {
        self.mtime
    }
}

impl<R: Read + Seek> Converter<R> {
    pub fn new<P: AsRef<Path>>(
        archive: zip::ZipArchive<R>,
        archive_path: PathBuf,
        outfile: P,
        concat_mode: arx::create::ConcatMode,
    ) -> jbk::Result<Self> {
        let progress = Arc::new(ProgressBar::new(&archive)?);
        let arx_creator = arx::create::SimpleCreator::new(
            outfile,
            concat_mode,
            Arc::clone(&progress) as Arc<dyn jbk::creator::Progress>,
            Rc::new(()),
            jbk::creator::Compression::zstd(),
        )?;

        Ok(Self {
            arx_creator,
            archive,
            archive_path,
            progress,
        })
    }

    fn finalize(self, outfile: &Path) -> jbk::Result<()> {
        self.arx_creator.finalize(outfile)
    }

    pub fn run(mut self, outfile: &Path) -> jbk::Result<()> {
        for idx in 0..self.archive.len() {
            self.progress.entries.inc(1);
            let entry = self.archive.by_index(idx).unwrap();
            let entry = ZipEntry::new(entry, self.arx_creator.adder(), &self.archive_path)?;
            self.arx_creator.add_entry(&entry)?;
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

    if args.generate_man_page {
        let man = clap_mangen::Man::new(Cli::command());
        man.render(&mut std::io::stdout())?;
        return Ok(());
    }

    let file = std::fs::File::open(args.zip_file.as_ref().unwrap())?;
    let archive = zip::ZipArchive::new(file).unwrap();
    let converter = Converter::new(
        archive,
        args.zip_file.unwrap(),
        args.outfile.as_ref().unwrap(),
        args.concat_mode.into(),
    )?;
    converter.run(args.outfile.as_ref().unwrap())
}
