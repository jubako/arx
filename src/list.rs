use crate::common::{Entry, EntryKind};
use jubako as jbk;
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct Lister {
    index: jbk::reader::Index,
    directory: Rc<jbk::reader::DirectoryPack>
}

impl Lister {
    fn new<P: AsRef<Path>>(infile: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&infile)?;
        let directory = container.get_directory_pack()?;
        let index = directory.get_index_from_name("root")?;
        Ok(Self{
            index,
            directory: Rc::clone(directory)
        })
    }

    fn list_all(&self) -> jbk::Result<()> {
        let entry_count = self.index.entry_count();
        println!("Found {} entries", entry_count);
        self.list_range(0, entry_count.0, PathBuf::new())
    }

    fn list_range(&self, start:u32, end:u32, currentPath: PathBuf) -> jbk::Result<()> {
        let key_storage = self.directory.get_key_storage();
        for idx in start..end {
            let entry = Entry::new(self.index.get_entry(jbk::Idx(idx))?, &key_storage);
            let path = currentPath.join(entry.get_path()?);
            println!("{}", path.display());
            if entry.get_type() == EntryKind::Directory {
                let first_child = entry.get_first_child().0;
                let nb_children = entry.get_nb_children().0;
                self.list_range(first_child, first_child+nb_children, path)?;
            }
        }
        Ok(())
    }
}


pub fn list<P: AsRef<Path>>(infile: P) -> jbk::Result<()> {
    let lister = Lister::new(infile)?;
    lister.list_all()
}
