use crate::common::{Entry, EntryKind};
use jbk::reader::EntryStoreTrait;
use jubako as jbk;
//use jbk::reader::Finder;
//use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack();
    let value_storage = directory.create_value_storage();
    let entry_storage = directory.create_entry_storage();
    let index = directory.get_index_from_name("root")?;
    let store = index.get_store::<jbk::reader::AnySchema>(&entry_storage)?;
    let resolver = jbk::reader::Resolver::new(Rc::clone(&value_storage));
    let mut current: Option<jbk::EntryIdx> = None;
    for component in path.as_ref().iter() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let finder = match current {
            None => index.get_finder(&entry_storage)?,
            Some(c) => {
                let parent = Entry::new(c, store.get_entry(c)?, Rc::clone(&value_storage));
                if !parent.is_dir() {
                    return Err("Cannot found entry".to_string().into());
                }
                let offset = parent.get_first_child();
                let count = parent.get_nb_children();
                jbk::reader::Finder::new(Rc::clone(&store), offset, count)
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
        let entry = Entry::new(idx, store.get_entry(idx)?, Rc::clone(&value_storage));
        match entry.get_type() {
            EntryKind::Directory => Err("Found directory".to_string().into()),
            EntryKind::File => {
                let content_address = entry.get_content_address();
                let reader = container.get_reader(&content_address)?;
                std::io::copy(
                    &mut reader.create_stream_all(),
                    &mut std::io::stdout().lock(),
                )?;
                Ok(())
            }
            EntryKind::Link => Err("Found link".to_string().into()),
        }
    } else {
        Err("Cannot found entry".to_string().into())
    }
}
