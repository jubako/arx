use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::path::Path;

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

fn dump_entry(
    container: &jbk::reader::Container,
    entry: &libarx::Entry<(jbk::ContentAddress, (), ())>,
) -> jbk::Result<()> {
    match entry {
        libarx::Entry::Dir(_, _) => Err("Found directory".to_string().into()),
        libarx::Entry::File(content_address) => {
            let reader = container.get_reader(*content_address)?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
            Ok(())
        }
        libarx::Entry::Link(_) => Err("Found link".to_string().into()),
    }
}

type FullBuilder = (FileBuilder, (), ());

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let arx = libarx::Arx::new(infile)?;
    dump_entry(
        &arx.container,
        &libarx::locate::<P, FullBuilder>(&arx, path)?,
    )
}
