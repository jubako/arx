use jubako as jbk;

use crate::common::EntryType;
use jbk::creator::schema;
use std::collections::{hash_map::Entry as MapEntry, HashMap};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
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
    parent: jbk::Word<u64>,
    is_root: bool,
}

impl Entry {
    pub fn new_root(path: PathBuf) -> jbk::Result<Self> {
        Self::new(path, ((|| 0) as fn() -> u64).into(), true)
    }

    fn new(path: PathBuf, parent: jbk::Word<u64>, is_root: bool) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: EntryKind::Dir,
                path,
                parent,
                is_root,
            }
        } else if attr.is_file() {
            Self {
                kind: EntryKind::File,
                path,
                parent,
                is_root,
            }
        } else if attr.is_symlink() {
            Self {
                kind: EntryKind::Link,
                path,
                parent,
                is_root,
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                parent,
                is_root,
            }
        })
    }

    pub fn new_from_fs(dir_entry: fs::DirEntry, parent: jbk::Word<u64>, is_root: bool) -> Self {
        let path = dir_entry.path();
        if let Ok(file_type) = dir_entry.file_type() {
            if file_type.is_dir() {
                Self {
                    kind: EntryKind::Dir,
                    path,
                    parent,
                    is_root,
                }
            } else if file_type.is_file() {
                Self {
                    kind: EntryKind::File,
                    path,
                    parent,
                    is_root,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: EntryKind::Link,
                    path,
                    parent,
                    is_root,
                }
            } else {
                Self {
                    kind: EntryKind::Other,
                    path,
                    parent,
                    is_root,
                }
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                parent,
                is_root,
            }
        }
    }
}

pub struct Creator {
    content_pack: jbk::creator::ContentPackCreator,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store: Box<jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>>,
    entry_count: jbk::EntryCount,
    root_count: jbk::EntryCount,
    content_cache: HashMap<blake3::Hash, jbk::ContentIdx>,
}

impl Creator {
    pub fn new<P: AsRef<Path>>(outfile: P) -> jbk::Result<Self> {
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
        )?;

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

        let entry_def = schema::Schema::new(
            // Common part
            schema::CommonProperties::new(vec![
                schema::Property::new_array(1, Rc::clone(&path_store)), // the path
                schema::Property::new_uint(),                           // index of the parent entry
                schema::Property::new_uint(),                           // owner
                schema::Property::new_uint(),                           // group
                schema::Property::new_uint(),                           // rights
                schema::Property::new_uint(),                           // modification time
            ]),
            vec![
                // File
                schema::VariantProperties::new(vec![
                    schema::Property::new_content_address(),
                    schema::Property::new_uint(), // Size
                ]),
                // Directory
                schema::VariantProperties::new(vec![
                    schema::Property::new_uint(), // index of the first entry
                    schema::Property::new_uint(), // nb entries in the directory
                ]),
                // Link
                schema::VariantProperties::new(vec![
                    schema::Property::new_array(1, Rc::clone(&path_store)), // Id of the linked entry
                ]),
            ],
            Some(vec![1.into(), 0.into()]),
        );

        let entry_store = Box::new(jbk::creator::EntryStore::new(entry_def));

        Ok(Self {
            content_pack,
            directory_pack,
            entry_store,
            entry_count: 0.into(),
            root_count: 0.into(),
            content_cache: HashMap::new(),
        })
    }

    pub fn finalize(mut self, outfile: PathBuf) -> jbk::Result<()> {
        let entry_store_id = self.directory_pack.add_entry_store(self.entry_store);
        self.directory_pack.create_index(
            "arx_entries",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            self.entry_count,
            jubako::EntryIdx::from(0),
        );
        self.directory_pack.create_index(
            "arx_root",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
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

    pub fn handle(&mut self, entry: Entry) -> jbk::Result<Option<jbk::Bound<jbk::EntryIdx>>> {
        if let EntryKind::Other = entry.kind {
            return Ok(None);
        };

        if self.entry_count.into_u32() % 1000 == 0 {
            println!("{}", self.entry_count);
        }

        if entry.is_root {
            self.root_count += 1;
        }

        let entry_path =
            jbk::Value::Array(entry.path.file_name().unwrap().to_os_string().into_vec());
        let metadata = fs::symlink_metadata(&entry.path)?;
        let entry = Box::new(match entry.kind {
            EntryKind::Dir => {
                let entry_idx = jbk::Vow::new(jbk::EntryIdx::from(0));

                let mut children_idx = Vec::new();
                for sub_entry in fs::read_dir(&entry.path)? {
                    let entry_idx_bind = entry_idx.bind();
                    let parent_idx_generator: Box<dyn Fn() -> u64> =
                        Box::new(move || entry_idx_bind.get().into_u64() + 1);
                    let child_idx = self.handle(Entry::new_from_fs(
                        sub_entry?,
                        parent_idx_generator.into(),
                        false,
                    ))?;
                    if let Some(child_idx) = child_idx {
                        children_idx.push(child_idx);
                    }
                }

                let entry_count = children_idx.len() as u64;
                let first_entry_generator: Box<dyn Fn() -> u64> = Box::new(move || {
                    children_idx
                        .iter()
                        .map(|i| i.get().into_u64())
                        .min()
                        .unwrap_or(0)
                });

                jbk::creator::BasicEntry::new_from_schema_idx(
                    &self.entry_store.schema,
                    entry_idx,
                    Some(EntryType::Dir.into()),
                    vec![
                        entry_path,
                        jbk::Value::Unsigned(entry.parent),
                        jbk::Value::Unsigned((metadata.uid() as u64).into()),
                        jbk::Value::Unsigned((metadata.gid() as u64).into()),
                        jbk::Value::Unsigned((metadata.mode() as u64).into()),
                        jbk::Value::Unsigned((metadata.mtime() as u64).into()),
                        jbk::Value::Unsigned(first_entry_generator.into()),
                        jbk::Value::Unsigned(entry_count.into()),
                    ],
                )
            }
            EntryKind::File => {
                let content_id =
                    self.add_content(jbk::creator::FileSource::open(&entry.path)?.into())?;
                jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::File.into()),
                    vec![
                        entry_path,
                        jbk::Value::Unsigned(entry.parent),
                        jbk::Value::Unsigned((metadata.uid() as u64).into()),
                        jbk::Value::Unsigned((metadata.gid() as u64).into()),
                        jbk::Value::Unsigned((metadata.mode() as u64).into()),
                        jbk::Value::Unsigned((metadata.mtime() as u64).into()),
                        jbk::Value::Content(jbk::ContentAddress::new(
                            jbk::PackId::from(1),
                            content_id,
                        )),
                        jbk::Value::Unsigned(metadata.size().into()),
                    ],
                )
            }
            EntryKind::Link => {
                let target = fs::read_link(&entry.path)?;
                jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::Link.into()),
                    vec![
                        entry_path,
                        jbk::Value::Unsigned(entry.parent),
                        jbk::Value::Unsigned((metadata.uid() as u64).into()),
                        jbk::Value::Unsigned((metadata.gid() as u64).into()),
                        jbk::Value::Unsigned((metadata.mode() as u64).into()),
                        jbk::Value::Unsigned((metadata.mtime() as u64).into()),
                        jbk::Value::Array(target.into_os_string().into_vec()),
                    ],
                )
            }
            EntryKind::Other => unreachable!(),
        });
        let current_idx = self.entry_store.add_entry(entry);
        self.entry_count += 1;
        Ok(Some(current_idx))
    }

    fn add_content(&mut self, content: jbk::Reader) -> jbk::Result<jbk::ContentIdx> {
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut content.create_flux_all(), &mut hasher)?;
        let hash = hasher.finalize();
        match self.content_cache.entry(hash) {
            MapEntry::Vacant(e) => {
                let content_idx = self.content_pack.add_content(content)?;
                e.insert(content_idx);
                Ok(content_idx)
            }
            MapEntry::Occupied(e) => Ok(*e.get()),
        }
    }
}
