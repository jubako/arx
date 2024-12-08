use arx::FileFilter;
use clap::{Parser, ValueHint};
use log::info;
use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

const AFTER_LONG_HELP: &str = color_print::cstr!("
<s,u>Filters</>

Arx support three kinds of filter.
- Simple values given as [EXTRACT_FILES].
- Glob given with <s>--glob</> option.
- File list given with <s>--file-list</> option.

All filters are exclusives. You can pass only one kind of filter.
Filters are relative to root directory.
Filters are only filtering entries to extract. No renaming is made.
If not filter is given, all entries under root directory are accepted.

<u>EXTRACT_FILES</>

List files to extracts.
This filter is recursive. If you give a directory, all files/subdirctory in the given
directory will also be extracted.

<u>GLOB</>

A glob pattern to match files/directory to extract.
This filter is not recursive. If you want to extract all file under a directory foo, use <K!>foo/**/*</>

- <K!>?</> matches any single character.
- <K!>*</> matches any (possibly empty) sequence of characters.
- <K!>**</> matches the current directory and arbitrary subdirectories. This sequence must form a single path component, so both <K!>**a</> and <K!>b**</> are invalid and will result in an error. A sequence of more than two consecutive <K!>*</> characters is also invalid.
- <K!>[...]</> matches any character inside the brackets. Character sequences can also specify ranges of characters, as ordered by Unicode, so e.g. <K!>[0-9]</> specifies any character between 0 and 9 inclusive. An unclosed bracket is invalid.
- <K!>[!...]</> is the negation of <K!>[...]</>, i.e. it matches any characters not in the brackets.
- The metacharacters <K!>?</>, <K!>*</>, <K!>[</>, <K!>]</> can be matched by using brackets (e.g. <K!>[?]</>). When a <K!>]</> occurs immediately following <K!>[</> or <K!>[!</> then it is interpreted as being part of, rather then ending, the character set, so <K!>]</> and NOT <K!>]</> can be matched by <K!>[]]</> and <K!>[!]]</> respectively. The <K!>-</> character can be specified inside a character sequence pattern by placing it at the start or the end, e.g. <K!>[abc-]</>.

<u>FILE_LIST</>

A plain file listing all files/directory to extract (one per line).
This filter is not recursive.
This filter <i>early exits</>. You must give all parent directory to extract a file.


<s,u>Root Directory</>

By default, arx extracts from the root directory of the archive.
<s>--root-dir</> option allow to change the root directory.
This is equivalent to a (virtual) cd in the root directory before walking the tree and apply filter.");

/// Extract the content of an archive
#[derive(Parser, Debug)]
#[command(after_long_help=AFTER_LONG_HELP)]
pub struct Options {
    /// Archive to read
    #[arg(value_hint=ValueHint::FilePath)]
    infile: PathBuf,

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

    /// Use a glob pattern to filter file to extract
    #[arg(short = 'g', long, group = "input")]
    glob: Option<String>,

    #[arg(from_global)]
    verbose: u8,

    #[arg(long, default_value = "warn")]
    overwrite: arx::Overwrite,
}

fn get_extract_filter(options: &Options) -> anyhow::Result<Box<dyn FileFilter>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list)?;
        let mut files: HashSet<arx::PathBuf> = Default::default();
        for line in BufReader::new(file).lines() {
            files.insert(line?.into());
        }
        Ok(Box::new(files))
    } else if let Some(pattern) = &options.glob {
        Ok(Box::new(PatternFilter(glob::Pattern::new(pattern)?)))
    } else if !options.extract_files.is_empty() {
        Ok(Box::new(SimpleFileList(options.extract_files.clone())))
    } else {
        Ok(Box::new(()))
    }
}

struct PatternFilter(pub glob::Pattern);

impl arx::FileFilter for PatternFilter {
    fn accept(&self, path: &arx::Path) -> bool {
        const MATCH_OPTIONS: glob::MatchOptions = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };
        self.0.matches_with(path.as_str(), MATCH_OPTIONS)
    }

    fn early_exit(&self) -> bool {
        false
    }
}

struct SimpleFileList(pub Vec<arx::PathBuf>);

impl arx::FileFilter for SimpleFileList {
    fn accept(&self, path: &arx::Path) -> bool {
        for accepted_path in &self.0 {
            if accepted_path == path || accepted_path.starts_with(path) {
                return true;
            }
        }
        false
    }

    fn early_exit(&self) -> bool {
        false
    }
}

type DummyBuilder = ((), (), ());

pub fn extract(options: Options) -> anyhow::Result<()> {
    let filter = get_extract_filter(&options)?;
    let outdir = match options.outdir {
        Some(o) => o,
        None => current_dir()?,
    };

    let arx = arx::Arx::new(&options.infile)?;
    info!("Extract archive {:?} in {:?}", &options.infile, outdir);

    match options.root_dir {
        None => arx::extract_arx(&arx, &outdir, filter, options.progress, options.overwrite)?,
        Some(p) => {
            let relative_path = arx::Path::from_path(&p)?;
            let root = arx.get_entry::<DummyBuilder>(relative_path)?;
            match root {
                arx::Entry::Dir(range, _) => arx::extract_arx_range(
                    &arx,
                    &outdir,
                    &range,
                    filter,
                    options.progress,
                    options.overwrite,
                )?,
                _ => return Err(anyhow::anyhow!("{} must be a directory", p.display())),
            }
        }
    };
    Ok(())
}
