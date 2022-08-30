use crate::entry::{Entry, EntryKind};
use jubako as jbk;
use std::fs::{create_dir, create_dir_all, File};
use std::os::unix::fs::symlink;
use std::path::Path;

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index(0.into())?;
    let key_storage = directory.get_key_storage();
    let entry_count = index.entry_count();
    create_dir_all(&outdir)?;
    for idx in 0..entry_count.0 {
        let entry = Entry::new(index.get_entry(jbk::Idx(idx))?, &key_storage);
        let target_path = outdir.as_ref().join(Path::new(&entry.get_path()?));
        match &entry.get_type() {
            EntryKind::File => {
                let content_address = entry.get_content_address();
                let reader = container.get_reader(content_address)?;
                let mut target_file = File::create(target_path)?;
                std::io::copy(&mut reader.create_stream_all(), &mut target_file)?;
            }
            EntryKind::Directory => {
                create_dir(target_path)?;
            }
            EntryKind::Link => {
                let target = entry.get_target_link()?;
                symlink(target, target_path)?;
            }
        }
    }
    Ok(())
}
