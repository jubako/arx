use crate::common::{Entry, EntryCompare, Schema};
use jbk::reader::schema::SchemaTrait;
use jubako as jbk;
use std::path::Path;
use std::rc::Rc;

fn dump_entry(container: &jbk::reader::Container, entry: &Entry) -> jbk::Result<()> {
    match entry {
        Entry::Dir(_) => Err("Found directory".to_string().into()),
        Entry::File(e) => {
            let reader = container.get_reader(e.get_content_address())?;
            std::io::copy(
                &mut reader.create_stream_all(),
                &mut std::io::stdout().lock(),
            )?;
            Ok(())
        }
        Entry::Link(_) => Err("Found link".to_string().into()),
    }
}

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let schema = Schema::new(&container);
    let directory = container.get_directory_pack();
    let value_storage = directory.create_value_storage();
    let entry_storage = directory.create_entry_storage();
    let index = directory.get_index_from_name("root")?;
    let builder = schema.create_builder(index.get_store(&entry_storage)?)?;
    let resolver = jbk::reader::Resolver::new(Rc::clone(&value_storage));
    let mut current_finder: jbk::reader::Finder<Schema> = index.get_finder(&builder)?;
    let mut components = path.as_ref().iter().peekable();
    while let Some(component) = components.next() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let comparator = EntryCompare::new(&resolver, &builder, component);
        let found = current_finder.find(&comparator)?;
        match found {
            None => return Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                let entry = current_finder.get_entry(idx)?;
                if components.peek().is_none() {
                    // We have the last component
                    return dump_entry(&container, &entry);
                } else if let Entry::Dir(e) = entry {
                    let offset = e.get_first_child();
                    let count = e.get_nb_children();
                    current_finder =
                        jbk::reader::Finder::new(current_finder.builder(), offset, count);
                } else {
                    return Err("Cannot found entry".to_string().into());
                }
            }
        }
    }

    unreachable!();
}
