use crate::common::{Arx, Entry, EntryCompare};
use jubako as jbk;
use jubako::reader::Range;
use std::path::Path;

fn dump_entry(container: &jbk::reader::Container, entry: &Entry) -> jbk::Result<()> {
    match entry {
        Entry::Dir(_) => Err("Found directory".to_string().into()),
        Entry::File(e) => {
            let reader = container.get_reader(e.get_content_address())?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
            Ok(())
        }
        Entry::Link(_) => Err("Found link".to_string().into()),
    }
}

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let root_index = arx.root_index()?;
    let builder = arx.create_builder(&root_index)?;
    let mut current_range: jbk::EntryRange = (&root_index).into();
    let mut components = path.as_ref().iter().peekable();
    while let Some(component) = components.next() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let comparator = EntryCompare::new(&builder, component);
        let found = current_range.find(&comparator)?;
        match found {
            None => return Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                let entry = current_range.get_entry(&builder, idx)?;
                if components.peek().is_none() {
                    // We have the last component
                    return dump_entry(&arx.container, &entry);
                } else if let Entry::Dir(e) = entry {
                    current_range = (&e).into();
                } else {
                    return Err("Cannot found entry".to_string().into());
                }
            }
        }
    }
    unreachable!();
}
