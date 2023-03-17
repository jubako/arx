use crate::common::{AllProperties, Arx, EntryCompare, EntryType};
use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use jubako::reader::Range;
use std::path::Path;
use std::rc::Rc;

enum Entry {
    File(jbk::ContentAddress),
    Link,
    Dir(jbk::EntryRange),
}

struct EntryBuilder {
    store: Rc<jbk::reader::EntryStore>,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl EntryBuilder {
    pub fn new(properties: &AllProperties) -> Self {
        Self {
            store: Rc::clone(&properties.store),
            variant_id_property: properties.variant_id_property,
            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
            content_address_property: properties.file_content_address_property,
        }
    }
}

impl jbk::reader::builder::BuilderTrait for EntryBuilder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let file_type = self.variant_id_property.create(&reader)?.try_into()?;
        Ok(match file_type {
            EntryType::File => {
                let content_address = self.content_address_property.create(&reader)?;
                Entry::File(content_address)
            }
            EntryType::Link => Entry::Link,
            EntryType::Dir => {
                let first_child: jbk::EntryIdx =
                    (self.first_child_property.create(&reader)? as u32).into();
                let nb_children: jbk::EntryCount =
                    (self.nb_children_property.create(&reader)? as u32).into();
                let range = jbk::EntryRange::new(first_child, nb_children);
                Entry::Dir(range)
            }
        })
    }
}

fn dump_entry(container: &jbk::reader::Container, entry: &Entry) -> jbk::Result<()> {
    match entry {
        Entry::Dir(_) => Err("Found directory".to_string().into()),
        Entry::File(content_address) => {
            let reader = container.get_reader(*content_address)?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
            Ok(())
        }
        Entry::Link => Err("Found link".to_string().into()),
    }
}

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let root_index = arx.root_index()?;
    let properties = arx.create_properties(&root_index)?;
    let builder = EntryBuilder::new(&properties);
    let mut current_range: jbk::EntryRange = (&root_index).into();
    let mut components = path.as_ref().iter().peekable();
    while let Some(component) = components.next() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let comparator = EntryCompare::new(&properties, component);
        let found = current_range.find(&comparator)?;
        match found {
            None => return Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                let entry = current_range.get_entry(&builder, idx)?;
                if components.peek().is_none() {
                    // We have the last component
                    return dump_entry(&arx.container, &entry);
                } else if let Entry::Dir(range) = entry {
                    current_range = range;
                } else {
                    return Err("Cannot found entry".to_string().into());
                }
            }
        }
    }
    unreachable!();
}
