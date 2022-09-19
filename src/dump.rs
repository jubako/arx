use crate::entry::{Entry, EntryKind};
use jubako as jbk;
use std::path::Path;
use std::ffi::OsStr;

pub fn dump<P: AsRef<Path>>(infile: P, path: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index_from_name("root")?;
    let key_storage = directory.get_key_storage();
    let mut current_parent: Option<jbk::Idx<u32>> = None;
    let mut min = 0;
    let mut max = index.entry_count().0;
    let mut found: Option<jbk::Idx<u32>> = None;
    for component in path.as_ref().iter() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let mut idx = min;
        loop {
            if idx == max {
                return Err("Cannot found entry".to_string().into())
            }
            let entry = Entry::new(index.get_entry(jbk::Idx(idx))?, &key_storage);
            if entry.get_parent() != current_parent {
                return Err("Cannot found entry".to_string().into())
            }
            let entry_path = entry.get_path()?;
            let entry_path: &OsStr = entry_path.as_ref();
            if entry_path == component {
                // We have found the entry of the componnent
                found = Some(jbk::Idx(idx));
                if entry.get_type() == EntryKind::Directory {
                    min = entry.get_first_child().0;
                    max = min+entry.get_nb_children().0;
                    current_parent = Some(jbk::Idx(idx));
                }
                break;
            }
            idx += 1;
        }
    }

    if let Some(idx) = found {
        let entry = Entry::new(index.get_entry(idx)?, &key_storage);
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
            },
            EntryKind::Link => Err("Found link".to_string().into())
        }
    } else {
        Err("Cannot found entry".to_string().into())
    }
}
