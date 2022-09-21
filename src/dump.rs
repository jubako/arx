use crate::common::{Entry, EntryKind};
use jubako as jbk;
//use jbk::reader::Finder;
//use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index_from_name("root")?;
    let resolver = directory.get_resolver();
    let mut current: Option<Entry> = None;
    for component in path.as_ref().iter() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let finder = match current {
            None => index.get_finder(Rc::clone(&resolver)),
            Some(c) => {
                if !c.is_dir() {
                    return Err("Cannot found entry".to_string().into());
                }
                let offset = c.get_first_child();
                let count = c.get_nb_children();
                jbk::reader::Finder::new(index.get_store(), offset, count, Rc::clone(&resolver))
            }
        };
        let entry = finder.find(
            0,
            jbk::reader::Value::Array(component.to_os_string().into_vec()),
        )?;
        match entry {
            None => {
                return Err("Cannot found entry".to_string().into());
            }
            Some(entry) => {
                current = Some(Entry::new(entry, Rc::clone(&resolver)));
            }
        }
    }

    if let Some(entry) = current {
        match entry.get_type() {
            EntryKind::Directory => Err("Found directory".to_string().into()),
            EntryKind::File => {
                let content_address = entry.get_content_address();
                let reader = container.get_reader(content_address)?;
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
