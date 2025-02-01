use clap::{CommandFactory, Parser, ValueHint};
use jbk::creator::ContentAdder;

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
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

/// Convert a zip archive into an Arx archive.
#[derive(Parser)]
#[command(name = "tar2arx", author, version, long_version=VERSION, about, long_about=None)]
struct Cli {
    /// Zip file to convert
    #[arg(
        value_parser,
        required_unless_present_any(
            ["list_compressions", "generate_man_page", "generate_complete"]
        ),
        value_hint=ValueHint::FilePath
    )]
    zip_file: Option<PathBuf>,

    /// Archive name to create
    #[arg(
        short,
        long,
        value_parser,
        required_unless_present_any(
            ["list_compressions", "generate_man_page", "generate_complete"]
        ),
        value_hint=ValueHint::FilePath
    )]
    outfile: Option<jbk::Utf8PathBuf>,

    #[command(flatten)]
    concat_mode: Option<jbk::cmd_utils::ConcatMode>,

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

    #[arg(long, help_heading = "Advanced")]
    generate_man_page: bool,

    #[arg(long, help_heading = "Advanced")]
    generate_complete: Option<clap_complete::Shell>,
}

#[derive(Clone)]
struct ProgressBar {
    pub comp_clusters: indicatif::ProgressBar,
    pub uncomp_clusters: indicatif::ProgressBar,
    pub entries: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new<R: Read + Seek>(archive: &zip::ZipArchive<R>) -> Self {
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
        Self {
            comp_clusters,
            uncomp_clusters,
            entries,
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
    pub fn new(
        mut entry: zip::read::ZipFile<'_>,
        adder: &mut impl ContentAdder,
        archive_path: &Path,
    ) -> Result<Self, arx::CreatorError> {
        let mut mtime = None;
        for extra_field in entry.extra_data_fields() {
            match extra_field {
                zip::ExtraField::ExtendedTimestamp(ex_timestamp) => {
                    mtime = ex_timestamp.mod_time().map(|ts| ts as u64)
                }
            }
        }
        let mtime = match mtime {
            Some(ts) => ts,
            None => entry
                .last_modified()
                .map(|ts| time::OffsetDateTime::try_from(ts).unwrap().unix_timestamp() as u64)
                .unwrap_or(0),
        };
        let mode = entry.unix_mode().unwrap_or(0o644) as u64;
        let path = entry
            .enclosed_name()
            .ok_or(arx::InputError("Invalid path".into()))?;
        let path =
            arx::PathBuf::from_path(&path).unwrap_or_else(|_| panic!("{path:?} must be utf8"));

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
                adder.add_content(Box::new(reader), jbk::creator::CompHint::Detect)?
            } else {
                let mut data = vec![];
                entry.read_to_end(&mut data)?;
                adder.add_content(
                    Box::new(std::io::Cursor::new(data)),
                    jbk::creator::CompHint::Detect,
                )?
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
    fn kind(&self) -> Result<Option<arx::create::EntryKind>, arx::CreatorError> {
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
    pub fn new(
        archive: zip::ZipArchive<R>,
        archive_path: PathBuf,
        outfile: impl AsRef<jbk::Utf8Path>,
        concat_mode: jbk::creator::ConcatMode,
    ) -> Result<Self, arx::CreatorError> {
        let progress = Arc::new(ProgressBar::new(&archive));
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

    fn finalize(self) -> Result<(), arx::CreatorError> {
        self.arx_creator.finalize()
    }

    pub fn run(mut self) -> Result<(), arx::CreatorError> {
        for idx in 0..self.archive.len() {
            self.progress.entries.inc(1);
            let entry = self.archive.by_index(idx).unwrap();
            let entry = ZipEntry::new(entry, self.arx_creator.adder(), &self.archive_path)?;
            self.arx_creator.add_entry(&entry)?;
        }
        self.finalize()
    }
}

fn main() -> Result<(), arx::CreatorError> {
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

    let file = std::fs::File::open(args.zip_file.as_ref().unwrap())?;
    let archive = zip::ZipArchive::new(file).unwrap();
    let converter = Converter::new(
        archive,
        args.zip_file.unwrap(),
        args.outfile.as_ref().unwrap(),
        match args.concat_mode {
            None => jbk::creator::ConcatMode::OneFile,
            Some(e) => e.into(),
        },
    )?;
    converter.run()
}
