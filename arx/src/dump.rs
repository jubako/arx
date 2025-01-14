use arx::{ArxError, ArxFormatError};
use clap::{Parser, ValueHint};
use jbk::reader::ByteSlice;
use jbk::reader::{builder::PropertyBuilderTrait, MayMissPack};
use log::info;
use std::path::PathBuf;

struct FileBuilder {
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl arx::Builder for FileBuilder {
    type Entry = jbk::ContentAddress;

    fn new(properties: &arx::AllProperties) -> Self {
        Self {
            content_address_property: properties.file_content_address_property,
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        self.content_address_property.create(reader)
    }
}

type FullBuilder = (FileBuilder, (), ());

fn dump_entry(
    container: &jbk::reader::Container,
    entry: arx::Entry<(jbk::ContentAddress, (), ())>,
    output: &mut dyn std::io::Write,
) -> Result<(), ArxError> {
    match entry {
        arx::Entry::Dir(_, _) => Err(ArxError::IsADirectory),
        arx::Entry::File(content_address) => {
            match container
                .get_bytes(content_address)?
                .and_then(|m| m.transpose())
                .ok_or(ArxFormatError("Content address should be valid"))?
            {
                MayMissPack::FOUND(bytes) => {
                    std::io::copy(&mut bytes.stream(), output)?;
                }
                MayMissPack::MISSING(pack_info) => {
                    eprintln!(
                        "Missing pack {}. Declared location is {}",
                        pack_info.uuid,
                        String::from_utf8_lossy(&pack_info.pack_location)
                    );
                }
            }
            Ok(())
        }
        arx::Entry::Link(_) => Err(ArxError::IsALink),
    }
}

/// Print the content of an entry in the archive.
#[derive(Parser, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_parser, value_hint=ValueHint::FilePath)]
    infile: PathBuf,

    /// Path of the entry to print
    #[arg(value_parser)]
    path: arx::PathBuf,

    /// Output Path. If not present or -, print to stdout
    #[arg(value_parser, value_hint=ValueHint::FilePath)]
    output: Option<String>,

    #[arg(from_global)]
    verbose: u8,
}

pub fn dump(options: Options) -> Result<(), ArxError> {
    info!(
        "Dump entry {} in archive {:?}",
        options.path, options.infile
    );
    let arx = arx::Arx::new(options.infile)?;
    let entry = arx.get_entry::<FullBuilder>(&options.path)?;
    match options.output {
        None => dump_entry(&arx, entry, &mut std::io::stdout()),
        Some(out) => {
            if out == "-" {
                dump_entry(&arx, entry, &mut std::io::stdout())
            } else {
                dump_entry(&arx, entry, &mut std::fs::File::open(out)?)
            }
        }
    }
}
