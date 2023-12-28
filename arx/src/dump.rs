use jbk::reader::builder::PropertyBuilderTrait;
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

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &arx::Reader) -> jbk::Result<Self::Entry> {
        self.content_address_property.create(reader)
    }
}

type FullBuilder = (FileBuilder, (), ());

fn dump_entry(
    container: &jbk::reader::Container,
    entry: arx::Entry<(jbk::ContentAddress, (), ())>,
) -> jbk::Result<()> {
    match entry {
        arx::Entry::Dir(_, _) => Err("Found directory".to_string().into()),
        arx::Entry::File(content_address) => {
            let reader = container.get_reader(content_address)?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
            Ok(())
        }
        arx::Entry::Link(_) => Err("Found link".to_string().into()),
    }
}

/// Print the content of an entry in the archive.
#[derive(clap::Args, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_parser)]
    infile: PathBuf,

    /// Path of the entry to print
    #[arg(value_parser)]
    path: arx::PathBuf,

    #[arg(from_global)]
    verbose: u8,
}

pub fn dump(options: Options) -> jbk::Result<()> {
    info!(
        "Dump entry {} in archive {:?}",
        options.path, options.infile
    );
    let arx = arx::Arx::new(options.infile)?;
    dump_entry(&arx, arx.get_entry::<FullBuilder>(&options.path)?)
}
