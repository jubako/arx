use jubako as jbk;

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use typenum::{U31, U40, U63};

const VENDOR_ID: u32 = 0x41_52_57_00;

enum EntryKind {
    Dir,
    File,
    Link,
    Other,
}

pub struct Entry {
    kind: EntryKind,
    path: PathBuf,
}

impl Entry {
    pub fn new(path: PathBuf) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: EntryKind::Dir,
                path,
            }
        } else if attr.is_file() {
            Self {
                kind: EntryKind::File,
                path,
            }
        } else if attr.is_symlink() {
            Self {
                kind: EntryKind::Link,
                path,
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
            }
        })
    }
}

impl From<fs::DirEntry> for Entry {
    fn from(dir_entry: fs::DirEntry) -> Self {
        let path = dir_entry.path();
        if let Ok(file_type) = dir_entry.file_type() {
            if file_type.is_dir() {
                Self {
                    kind: EntryKind::Dir,
                    path,
                }
            } else if file_type.is_file() {
                Self {
                    kind: EntryKind::File,
                    path,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: EntryKind::Link,
                    path,
                }
            } else {
                Self {
                    kind: EntryKind::Other,
                    path,
                }
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
            }
        }
    }
}

pub struct Creator {
    content_pack: jbk::creator::ContentPackCreator,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store_id: jbk::Idx<u32>,
    entry_count: u32,
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
            jbk::Id(1),
            VENDOR_ID,
            jbk::FreeData::<U40>::clone_from_slice(&[0x00; 40]),
            jbk::CompressionType::Zstd,
        );

        outfilename = outfile.file_name().unwrap().to_os_string();
        outfilename.push(".jbkd");
        let mut directory_pack_path = PathBuf::new();
        directory_pack_path.push(outfile);
        directory_pack_path.set_file_name(outfilename);
        let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
            directory_pack_path,
            jbk::Id(0),
            VENDOR_ID,
            jbk::FreeData::<U31>::clone_from_slice(&[0x00; 31]),
        );

        let path_store = directory_pack.create_key_store(jbk::creator::KeyStoreKind::Plain);

        let entry_def = jbk::creator::Entry::new(vec![
            // File
            jbk::creator::Variant::new(vec![
                jbk::creator::Key::PString(0, Rc::clone(&path_store)),
                jbk::creator::Key::ContentAddress,
            ]),
            // Directory
            jbk::creator::Variant::new(vec![
                jbk::creator::Key::PString(0, Rc::clone(&path_store)),
                jbk::creator::Key::UnsignedInt(4), // nb entries in the directory
            ]),
            // Link
            jbk::creator::Variant::new(vec![
                jbk::creator::Key::PString(0, Rc::clone(&path_store)),
                jbk::creator::Key::PString(0, Rc::clone(&path_store)), // Id of the linked entry
            ]),
        ]);

        let entry_store_id = directory_pack.create_entry_store(entry_def);

        Self {
            content_pack,
            directory_pack,
            entry_store_id,
            entry_count: 0,
            queue: VecDeque::<Entry>::new(),
        }
    }

    pub fn start(&mut self) -> jbk::Result<()> {
        self.content_pack.start()?;
        Ok(())
    }

    pub fn finalize(&mut self, outfile: PathBuf) -> jbk::Result<()> {
        self.directory_pack.create_index(
            "entries",
            jubako::ContentAddress::new(0.into(), 0.into()),
            0.into(),
            self.entry_store_id,
            jubako::Count(self.entry_count),
            jubako::Idx(0),
        );
        let directory_pack_info = self.directory_pack.finalize()?;
        let content_pack_info = self.content_pack.finalize()?;
        let mut manifest_creator = jbk::creator::ManifestPackCreator::new(
            outfile,
            VENDOR_ID,
            jbk::FreeData::<U63>::clone_from_slice(&[0x00; 63]),
        );

        manifest_creator.add_pack(directory_pack_info);
        manifest_creator.add_pack(content_pack_info);
        manifest_creator.finalize()?;
        Ok(())
    }

    pub fn push_back(&mut self, entry: Entry) {
        self.queue.push_back(entry);
    }

    pub fn run(&mut self) -> jbk::Result<()> {
        while !self.queue.is_empty() {
            let entry = self.queue.pop_front().unwrap();
            self.handle(entry)?;
            if self.entry_count % 1000 == 0 {
                println!("{}", self.entry_count);
            }
        }
        Ok(())
    }

    fn handle(&mut self, entry: Entry) -> jbk::Result<()> {
        let entry_path = jbk::creator::Value::Array {
            data: entry.path.as_os_str().to_os_string().into_vec(),
            key_id: None,
        };
        match entry.kind {
            EntryKind::Dir => {
                let mut nb_entries = 0;
                for sub_entry in fs::read_dir(&entry.path)? {
                    self.push_back(sub_entry?.into());
                    nb_entries += 1;
                }
                let entry_store = self.directory_pack.get_entry_store(self.entry_store_id);
                entry_store.add_entry(
                    1,
                    vec![entry_path, jbk::creator::Value::Unsigned(nb_entries)],
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
                        jbk::creator::Value::Content(jubako::ContentAddress::new(
                            jbk::Id(1),
                            content_id,
                        )),
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
                        jbk::creator::Value::Array {
                            data: target.into_os_string().into_vec(),
                            key_id: None,
                        },
                    ],
                );
                self.entry_count += 1;
            }
            EntryKind::Other => {}
        }
        Ok(())
    }
}
