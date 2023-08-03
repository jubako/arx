use jbk::reader::builder::PropertyBuilderTrait;
use std::path::PathBuf;

struct FileBuilder {
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl libarx::Builder for FileBuilder {
    type Entry = jbk::ContentAddress;

    fn new(properties: &libarx::AllProperties) -> Self {
        Self {
            content_address_property: properties.file_content_address_property,
        }
    }

    fn create_entry(
        &self,
        _idx: jbk::EntryIdx,
        reader: &libarx::Reader,
    ) -> jbk::Result<Self::Entry> {
        self.content_address_property.create(reader)
    }
}

type FullBuilder = (FileBuilder, (), ());

fn dump_entry(
    container: &jbk::reader::Container,
    entry: libarx::Entry<(jbk::ContentAddress, (), ())>,
) -> jbk::Result<()> {
    match entry {
        libarx::Entry::Dir(_, _) => Err("Found directory".to_string().into()),
        libarx::Entry::File(content_address) => {
            let reader = container.get_reader(content_address)?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
            Ok(())
        }
        libarx::Entry::Link(_) => Err("Found link".to_string().into()),
    }
}

#[derive(clap::Args)]
pub struct Options {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    path: String,
}

pub fn dump(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!(
            "Dump entry {} in archive {:?}",
            options.path, options.infile
        );
    }
    let arx = libarx::Arx::new(options.infile)?;
    dump_entry(&arx, arx.get_entry::<FullBuilder, _>(options.path)?)
}
