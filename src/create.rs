use jubako as jbk;

use jbk::creator::layout;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const VENDOR_ID: u32 = 0x41_52_58_00;

enum EntryKind {
    Dir,
    File,
    Link,
    Other,
}

pub struct Entry {
    kind: EntryKind,
    path: PathBuf,
    parent: jbk::EntryIdx,
}

impl Entry {
    pub fn new(path: PathBuf, parent: jbk::EntryIdx) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: EntryKind::Dir,
                path,
                parent,
            }
        } else if attr.is_file() {
            Self {
                kind: EntryKind::File,
                path,
                parent,
            }
        } else if attr.is_symlink() {
            Self {
                kind: EntryKind::Link,
                path,
                parent,
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                parent,
            }
        })
    }

    pub fn new_from_fs(dir_entry: fs::DirEntry, parent: jbk::EntryIdx) -> Self {
        let path = dir_entry.path();
        if let Ok(file_type) = dir_entry.file_type() {
            if file_type.is_dir() {
                Self {
                    kind: EntryKind::Dir,
                    path,
                    parent,
                }
            } else if file_type.is_file() {
                Self {
                    kind: EntryKind::File,
                    path,
                    parent,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: EntryKind::Link,
                    path,
                    parent,
                }
            } else {
                Self {
                    kind: EntryKind::Other,
                    path,
                    parent,
                }
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                parent,
            }
        }
    }
}

pub struct Creator {
    content_pack: jbk::creator::ContentPackCreator,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store_id: jbk::EntryStoreIdx,
    entry_count: jbk::EntryCount,
    root_count: jbk::EntryCount,
    queue: VecDeque<Entry>,
}

impl Creator {
    pub fn new<P: AsRef<Path>>(outfile: P) -> Self {
        let outfile = outfile.as_ref();
        let mut outfilename: OsString = outfile.file_name().unwrap().to_os_string();
        outfilename.push(".jbkc");
        let mut content_pack_path = PathBuf::new();
        content_pack_path.push(outfile);
        content_pack_path.set_file_name(outfilename);
        let content_pack = jbk::creator::ContentPackCreator::new(
            content_pack_path,
            jbk::PackId::from(1),
            VENDOR_ID,
            jbk::FreeData40::clone_from_slice(&[0x00; 40]),
            jbk::CompressionType::Zstd,
        );

        outfilename = outfile.file_name().unwrap().to_os_string();
        outfilename.push(".jbkd");
        let mut directory_pack_path = PathBuf::new();
        directory_pack_path.push(outfile);
        directory_pack_path.set_file_name(outfilename);
        let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
            directory_pack_path,
            jbk::PackId::from(0),
            VENDOR_ID,
            jbk::FreeData31::clone_from_slice(&[0x00; 31]),
        );

        let path_store = directory_pack.create_value_store(jbk::creator::ValueStoreKind::Plain);

        let entry_def = layout::Entry::new(vec![
            // File
            layout::Variant::new(vec![
                layout::Property::VLArray(1, Rc::clone(&path_store)),
                layout::Property::new_int(), // index of the parent entry
                layout::Property::ContentAddress,
            ]),
            // Directory
            layout::Variant::new(vec![
                layout::Property::VLArray(1, Rc::clone(&path_store)),
                layout::Property::new_int(), // index of the parent entry
                layout::Property::new_int(), // index of the first entry
                layout::Property::new_int(), // nb entries in the directory
            ]),
            // Link
            layout::Variant::new(vec![
                layout::Property::VLArray(1, Rc::clone(&path_store)),
                layout::Property::new_int(), // index of the parent entry
                layout::Property::VLArray(1, Rc::clone(&path_store)), // Id of the linked entry
            ]),
        ]);

        let entry_store_id = directory_pack.create_entry_store(entry_def);

        Self {
            content_pack,
            directory_pack,
            entry_store_id,
            entry_count: 0.into(),
            root_count: 0.into(),
            queue: VecDeque::<Entry>::new(),
        }
    }

    fn finalize(&mut self, outfile: PathBuf) -> jbk::Result<()> {
        self.directory_pack.create_index(
            "entries",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            self.entry_store_id,
            self.entry_count,
            jubako::EntryIdx::from(0),
        );
        self.directory_pack.create_index(
            "root",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            self.entry_store_id,
            self.root_count,
            jubako::EntryIdx::from(0),
        );
        let directory_pack_info = self.directory_pack.finalize()?;
        let content_pack_info = self.content_pack.finalize()?;
        let mut manifest_creator = jbk::creator::ManifestPackCreator::new(
            outfile,
            VENDOR_ID,
            jbk::FreeData63::clone_from_slice(&[0x00; 63]),
        );

        manifest_creator.add_pack(directory_pack_info);
        manifest_creator.add_pack(content_pack_info);
        manifest_creator.finalize()?;
        Ok(())
    }

    pub fn push_back(&mut self, entry: Entry) {
        if let EntryKind::Other = entry.kind {
            // do not add other to the queue
        } else {
            self.queue.push_back(entry);
        }
    }

    fn next_id(&self) -> jbk::EntryCount {
        // Return the id that will be pushed back.
        // The id is the entry_count (entries already added) + the size of the queue (entries to add)
        self.entry_count + self.queue.len() as u32
    }

    pub fn run(&mut self, outfile: PathBuf) -> jbk::Result<()> {
        self.content_pack.start()?;
        self.root_count = (self.queue.len() as u32).into();
        while !self.queue.is_empty() {
            let entry = self.queue.pop_front().unwrap();
            self.handle(entry)?;
            if self.entry_count.into_u32() % 1000 == 0 {
                println!("{}", self.entry_count);
            }
        }
        self.finalize(outfile)
    }

    fn handle(&mut self, entry: Entry) -> jbk::Result<()> {
        let entry_path =
            jbk::creator::Value::Array(entry.path.file_name().unwrap().to_os_string().into_vec());
        match entry.kind {
            EntryKind::Dir => {
                let mut nb_entries = 0;
                let first_entry = self.next_id() + 1; // The current directory is not in the queue but not yet added we need to count it now.
                for sub_entry in fs::read_dir(&entry.path)? {
                    self.push_back(Entry::new_from_fs(
                        sub_entry?,
                        jbk::EntryIdx::from(self.entry_count.into_u32() + 1),
                    ));
                    nb_entries += 1;
                }
                let entry_store = self.directory_pack.get_entry_store(self.entry_store_id);
                entry_store.add_entry(
                    1,
                    vec![
                        entry_path,
                        jbk::creator::Value::Unsigned(entry.parent.into_u64()),
                        jbk::creator::Value::Unsigned(first_entry.into_u64()),
                        jbk::creator::Value::Unsigned(nb_entries),
                    ],
                );
                self.entry_count += 1;
            }
            EntryKind::File => {
                let file = fs::File::open(&entry.path)?;
                let mut file = jbk::creator::FileStream::new(file, jbk::End::None);
                let content_id = self.content_pack.add_content(&mut file)?;
                let entry_store = self.directory_pack.get_entry_store(self.entry_store_id);
                entry_store.add_entry(
                    0,
                    vec![
                        entry_path,
                        jbk::creator::Value::Unsigned(entry.parent.into_u64()),
                        jbk::creator::Value::Content(jbk::creator::Content::from((
                            jbk::PackId::from(1),
                            content_id,
                        ))),
                    ],
                );
                self.entry_count += 1;
            }
            EntryKind::Link => {
                let target = fs::read_link(&entry.path)?;
                let entry_store = self.directory_pack.get_entry_store(self.entry_store_id);
                entry_store.add_entry(
                    2,
                    vec![
                        entry_path,
                        jbk::creator::Value::Unsigned(entry.parent.into_u64()),
                        jbk::creator::Value::Array(target.into_os_string().into_vec()),
                    ],
                );
                self.entry_count += 1;
            }
            EntryKind::Other => unreachable!(),
        }
        Ok(())
    }
}
