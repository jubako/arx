use std::fmt;

/// List available compressions
pub fn list_compressions() {
    println!("Available compressions :");
    println!(" - None");
    #[cfg(feature = "lz4")]
    println!(" - lz4 (level 0->15)");
    #[cfg(feature = "lzma")]
    println!(" - lzma (level 0->9)");
    #[cfg(feature = "zstd")]
    println!(" - zstd (level -22->22)")
}

/// Parse the compression given in command line in to a jbk::creator::Compression
pub fn compression_arg_parser(s: &str) -> Result<jbk::creator::Compression, InvalidCompression> {
    let mut iter = s.splitn(2, '=');
    let compression = iter.next().unwrap().to_ascii_lowercase();
    let level = iter.next();
    Ok(match compression.as_str() {
        "none" => jbk::creator::Compression::None,
        #[cfg(feature = "lz4")]
        "lz4" => match level {
            None => jbk::creator::Compression::lz4(),
            Some(l) => jbk::creator::Compression::Lz4(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        #[cfg(feature = "lzma")]
        "lzma" => match level {
            None => jbk::creator::Compression::lzma(),
            Some(l) => jbk::creator::Compression::Lzma(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        #[cfg(feature = "zstd")]
        "zstd" => match level {
            None => jbk::creator::Compression::zstd(),
            Some(l) => jbk::creator::Compression::Zstd(match l.parse() {
                Ok(l) => l,
                Err(e) => return Err(InvalidCompression::Level(e.to_string())),
            }),
        },
        _ => return Err(InvalidCompression::Algorithm(compression)),
    })
}

#[derive(Debug)]
pub enum InvalidCompression {
    Level(String),
    Algorithm(String),
}

impl fmt::Display for InvalidCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Level(e) => write!(f, "Invalid compression level: {}", e),
            Self::Algorithm(e) => write!(f, "Invalid compression algorithm: {}", e),
        }
    }
}
impl std::error::Error for InvalidCompression {}

/// Parse different flags to select the concat mode
#[derive(clap::Args, Debug)]
#[group(required = false, multiple = false)]
pub struct ConcatMode {
    #[arg(short = '1', long, required = false, default_value_t = false, action)]
    /// Create only one file (default)
    one_file: bool,

    #[arg(short = '2', long, required = false, default_value_t = false, action)]
    /// Create two files (a content pack and other)
    two_files: bool,

    #[arg(short = 'N', long, required = false, default_value_t = false, action)]
    /// Create mulitples files (one per pack)
    multiple_files: bool,
}

impl From<ConcatMode> for jbk::creator::ConcatMode {
    fn from(opt: ConcatMode) -> Self {
        {
            let (one, two, multiple) = (opt.one_file, opt.two_files, opt.multiple_files);
            match (one, two, multiple) {
                (true, _, _) => jbk::creator::ConcatMode::OneFile,
                (_, true, _) => jbk::creator::ConcatMode::TwoFiles,
                (_, _, true) => jbk::creator::ConcatMode::NoConcat,
                _ => unreachable!(),
            }
        }
    }
}
