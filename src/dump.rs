use crate::common::{Entry, EntryCompare, Schema};
use jbk::reader::EntryStoreTrait;
use jubako as jbk;
//use jbk::reader::Finder;
//use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let schema = Schema::new(&container);
    let directory = container.get_directory_pack();
    let value_storage = directory.create_value_storage();
    let entry_storage = directory.create_entry_storage();
    let index = directory.get_index_from_name("root")?;
    let store = index.get_store(&entry_storage, &schema)?;
    let resolver = jbk::reader::Resolver::new(Rc::clone(&value_storage));
    let mut current: Option<jbk::EntryIdx> = None;
    for component in path.as_ref().iter() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let finder = match current {
            None => index.get_finder(&entry_storage, &schema)?,
            Some(c) => {
                let parent = store.get_entry(c)?;
                if let Entry::Dir(e) = parent {
                    let offset = e.get_first_child();
                    let count = e.get_nb_children();
                    jbk::reader::Finder::new(Rc::clone(&store), offset, count)
                } else {
                    return Err("Cannot found entry".to_string().into());
                }
            }
        };
        let comparator = jbk::reader::PropertyCompare::new(
            resolver.clone(),
            jbk::PropertyIdx::from(0),
            jbk::reader::Value::Array(component.to_os_string().into_vec()),
        );
        let found = finder.find(&comparator)?;
        match found {
            None => return Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                current = Some(finder.offset() + idx);
            }
        }
    }

    if let Some(idx) = current {
        let entry = store.get_entry(idx)?;
        match entry {
            Entry::Dir(_) => Err("Found directory".to_string().into()),
            Entry::File(e) => {
                let content_address = e.get_content_address();
                let reader = container.get_reader(&content_address)?;
                std::io::copy(
                    &mut reader.create_stream_all(),
                    &mut std::io::stdout().lock(),
                )?;
                Ok(())
            }
            Entry::Link(_) => Err("Found link".to_string().into()),
        }
    } else {
        Err("Cannot found entry".to_string().into())
    }
}
