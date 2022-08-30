use crate::entry::Entry;
use jubako as jbk;
use std::path::Path;

pub fn list<P: AsRef<Path>>(infile: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index(0.into())?;
    let key_storage = directory.get_key_storage();
    let entry_count = index.entry_count();
    println!("Found {} entries", entry_count);
    for idx in 0..entry_count.0 {
        let entry = Entry::new(index.get_entry(jbk::Idx(idx))?, &key_storage);
        println!("{}", entry);
    }

    Ok(())
}
