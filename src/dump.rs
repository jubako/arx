use crate::entry::Entry;
use jubako as jbk;
use std::path::Path;

pub fn dump<P: AsRef<Path>>(infile: P, path: String) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index(0.into())?;
    let key_storage = directory.get_key_storage();
    let entry_count = index.entry_count();
    for idx in 0..entry_count.0 {
        let entry = Entry::new(index.get_entry(jbk::Idx(idx))?, &key_storage);
        if entry.get_path()? == path {
            let content_address = entry.get_content_address();
            let reader = container.get_reader(content_address)?;
            std::io::copy(
                &mut reader.create_stream_all(),
                &mut std::io::stdout().lock(),
            )?;
            return Ok(());
        }
    }
    Err("Cannot found entry".to_string().into())
}
