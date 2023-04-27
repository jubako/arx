use jubako as jbk;

use crate::common::EntryType;
use jbk::creator::schema;
use std::collections::{hash_map::Entry as MapEntry, HashMap};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const VENDOR_ID: u32 = 0x41_52_58_00;

#[derive(PartialEq, Eq, Debug)]
pub enum EntryKind {
    Dir,
    File(jbk::Size),
    Link,
    Other,
}

#[derive(Debug)]
pub struct Entry {
    pub kind: EntryKind,
    pub path: PathBuf,
    pub name: OsString,
    parent: jbk::Word<u64>,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl Entry {
    pub fn new_root(path: PathBuf) -> jbk::Result<Self> {
        let name = path.file_name().unwrap().to_os_string();
        Self::new(path, name, ((|| 0) as fn() -> u64).into())
    }

    fn new(path: PathBuf, name: OsString, parent: jbk::Word<u64>) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: EntryKind::Dir,
                path,
                name,
                parent,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else if attr.is_file() {
            Self {
                kind: EntryKind::File(attr.size().into()),
                path,
                name,
                parent,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else if attr.is_symlink() {
            Self {
                kind: EntryKind::Link,
                path,
                name,
                parent,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                name,
                parent,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        })
    }

    pub fn new_from_fs(dir_entry: fs::DirEntry, parent: jbk::Word<u64>) -> jbk::Result<Self> {
        let path = dir_entry.path();
        let name = dir_entry.file_name();
        Ok(if let Ok(file_type) = dir_entry.file_type() {
            let attr = fs::symlink_metadata(&path)?;
            if file_type.is_dir() {
                Self {
                    kind: EntryKind::Dir,
                    path,
                    name,
                    parent,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else if file_type.is_file() {
                Self {
                    kind: EntryKind::File(attr.size().into()),
                    path,
                    name,
                    parent,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: EntryKind::Link,
                    path,
                    name,
                    parent,

                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else {
                Self {
                    kind: EntryKind::Other,
                    path,
                    name,
                    parent,

                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            }
        } else {
            Self {
                kind: EntryKind::Other,
                path,
                name,
                parent,
                uid: 0,
                gid: 0,
                mode: 0,
                mtime: 0,
            }
        })
    }
}

type DirCache = HashMap<PathBuf, DirEntry>;
type EntryIdx = jbk::Bound<jbk::EntryIdx>;
type Void = jbk::Result<()>;

/// A DirEntry structure to keep track of added direcotry in the archive.
/// This is needed as we may adde file without recursion, and so we need
/// to find the parent of "foo/bar/baz.txt" ("foo/bar") when we add it.
struct DirEntry {
    idx: Option<EntryIdx>,
    dir_children: Rc<DirCache>,
    file_children: Rc<Vec<EntryIdx>>,
}

impl DirEntry {
    fn new_root() -> Self {
        Self {
            idx: None,
            dir_children: Default::default(),
            file_children: Default::default(),
        }
    }
    fn new(idx: EntryIdx) -> Self {
        Self {
            idx: Some(idx),
            dir_children: Default::default(),
            file_children: Default::default(),
        }
    }

    fn first_entry_generator(&self) -> Box<dyn Fn() -> u64> {
        let dir_children = Rc::clone(&self.dir_children);
        let file_children = Rc::clone(&self.file_children);
        Box::new(move || {
            if dir_children.is_empty() && file_children.is_empty() {
                0
            } else {
                std::cmp::min(
                    file_children
                        .iter()
                        .map(|i| i.get().into_u64())
                        .min()
                        .unwrap_or(u64::MAX),
                    dir_children
                        .values()
                        // Unwrap is safe because children are not root, and idx is Some
                        .map(|i| i.idx.as_ref().unwrap().get().into_u64())
                        .min()
                        .unwrap_or(u64::MAX),
                )
            }
        })
    }

    fn entry_count_generator(&self) -> Box<dyn Fn() -> u64> {
        let dir_children = Rc::clone(&self.dir_children);
        let file_children = Rc::clone(&self.file_children);
        Box::new(move || (dir_children.len() + file_children.len()) as u64)
    }

    fn as_parent_idx_generator(&self) -> Box<dyn Fn() -> u64> {
        match &self.idx {
            Some(idx) => {
                let idx = idx.clone();
                Box::new(move || idx.get().into_u64() + 1)
            }
            None => Box::new(|| 0),
        }
    }

    fn add_directory(
        &mut self,
        path: &Path,
        name: &OsStr,
        entry_store: &mut jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>,
    ) -> Void {
        if self.dir_children.contains_key(&PathBuf::from(name)) {
            return Ok(());
        }
        let metadata = fs::symlink_metadata(path)?;
        let entry_idx = jbk::Vow::new(jbk::EntryIdx::from(0));
        let entry_name = jbk::Value::Array(name.to_os_string().into_vec());

        let dir_entry = DirEntry::new(entry_idx.bind());

        let entry = Box::new(jbk::creator::BasicEntry::new_from_schema_idx(
            &entry_store.schema,
            entry_idx,
            Some(EntryType::Dir.into()),
            vec![
                entry_name,
                jbk::Value::Unsigned(self.as_parent_idx_generator().into()),
                jbk::Value::Unsigned((metadata.uid() as u64).into()),
                jbk::Value::Unsigned((metadata.gid() as u64).into()),
                jbk::Value::Unsigned((metadata.mode() as u64).into()),
                jbk::Value::Unsigned((metadata.mtime() as u64).into()),
                jbk::Value::Unsigned(dir_entry.first_entry_generator().into()),
                jbk::Value::Unsigned(dir_entry.entry_count_generator().into()),
            ],
        ));
        entry_store.add_entry(entry);
        /* SAFETY: We already have Rc on `self.dir_children` but it is only used
          in a second step to get entry_count and min entry_idx.
          So while we borrow `self.dir_children` we never read it otherwise.
        */
        unsafe { Rc::get_mut_unchecked(&mut self.dir_children) }
            .entry(name.into())
            .or_insert(dir_entry);
        Ok(())
    }

    fn mk_dirs(
        &mut self,
        mut path: PathBuf,
        mut components: std::path::Components,
        entry_store: &mut jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>,
    ) -> jbk::Result<&mut Self> {
        if let Some(component) = components.next() {
            path.push(component);
            if !self.dir_children.contains_key::<Path>(component.as_ref()) {
                self.add_directory(&path, component.as_os_str(), entry_store)?;
            }
            /* SAFETY: We already have Rc on `self.dir_children` but it is only used
              in a second step to get entry_count and min entry_idx.
              So while we borrow `self.dir_children` we never read it otherwise.
            */
            unsafe { Rc::get_mut_unchecked(&mut self.dir_children) }
                .get_mut::<Path>(component.as_ref())
                .unwrap()
                .mk_dirs(path, components, entry_store)
        } else {
            Ok(self)
        }
    }

    fn add<F, Adder>(
        &mut self,
        path: &Path,
        name: &OsStr,
        recurse: bool,
        filter: &F,
        entry_store: &mut jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>,
        add_content: &mut Adder,
    ) -> Void
    where
        F: Fn(Entry) -> Option<Entry>,
        Adder: FnMut(jbk::Reader) -> jbk::Result<jbk::ContentIdx>,
    {
        let entry = Entry::new(
            path.to_path_buf(),
            name.to_os_string(),
            self.as_parent_idx_generator().into(),
        )?;

        if let EntryKind::Other = entry.kind {
            return Ok(());
        };

        let entry = match filter(entry) {
            Some(e) => e,
            None => return Ok(()),
        };

        match entry.kind {
            EntryKind::Dir => {
                self.add_directory(path, &entry.name, entry_store)?;

                if recurse {
                    /* SAFETY: We already have Rc on `self.dir_children` but it is only used
                      in a second step to get entry_count and min entry_idx.
                      So while we borrow `self.dir_children` we never read it otherwise.
                    */
                    let dir_entry = unsafe { Rc::get_mut_unchecked(&mut self.dir_children) }
                        .get_mut::<Path>(entry.name.as_ref())
                        .unwrap();

                    for sub_entry in fs::read_dir(path)? {
                        let sub_entry = sub_entry?;
                        dir_entry.add(
                            &sub_entry.path(),
                            &sub_entry.file_name(),
                            recurse,
                            filter,
                            entry_store,
                            add_content,
                        )?;
                    }
                }
                Ok(())
            }
            EntryKind::File(size) => {
                let content_id = add_content(jbk::creator::FileSource::open(path)?.into())?;
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::File.into()),
                    vec![
                        jbk::Value::Array(entry.name.into_vec()),
                        jbk::Value::Unsigned(entry.parent),
                        jbk::Value::Unsigned(entry.uid.into()),
                        jbk::Value::Unsigned(entry.gid.into()),
                        jbk::Value::Unsigned(entry.mode.into()),
                        jbk::Value::Unsigned(entry.mtime.into()),
                        jbk::Value::Content(jbk::ContentAddress::new(
                            jbk::PackId::from(1),
                            content_id,
                        )),
                        jbk::Value::Unsigned(size.into_u64().into()),
                    ],
                ));
                let current_idx = entry_store.add_entry(entry);
                /* SAFETY: We already have Rc on `self.file_children` but it is only used
                  in a second step to get entry_count and min entry_idx.
                  So while we borrow `self.file_children` we never read it otherwise.
                */
                unsafe { Rc::get_mut_unchecked(&mut self.file_children) }.push(current_idx);
                Ok(())
            }
            EntryKind::Link => {
                let target = fs::read_link(path)?;
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::Link.into()),
                    vec![
                        jbk::Value::Array(entry.name.into_vec()),
                        jbk::Value::Unsigned(entry.parent),
                        jbk::Value::Unsigned(entry.uid.into()),
                        jbk::Value::Unsigned(entry.gid.into()),
                        jbk::Value::Unsigned(entry.mode.into()),
                        jbk::Value::Unsigned(entry.mtime.into()),
                        jbk::Value::Array(target.into_os_string().into_vec()),
                    ],
                ));
                let current_idx = entry_store.add_entry(entry);
                /* SAFETY: We already have Rc on `self.file_children` but it is only used
                  in a second step to get entry_count and min entry_idx.
                  So while we borrow `self.file_children` we never read it otherwise.
                */
                unsafe { Rc::get_mut_unchecked(&mut self.file_children) }.push(current_idx);
                Ok(())
            }
            EntryKind::Other => unreachable!(),
        }
    }
}

pub struct CachedContentPack {
    content_pack: jbk::creator::ContentPackCreator,
    cache: HashMap<blake3::Hash, jbk::ContentIdx>,
}

impl CachedContentPack {
    fn new(content_pack: jbk::creator::ContentPackCreator) -> Self {
        Self {
            content_pack,
            cache: Default::default(),
        }
    }

    fn add_content(&mut self, content: jbk::Reader) -> jbk::Result<jbk::ContentIdx> {
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut content.create_flux_all(), &mut hasher)?;
        let hash = hasher.finalize();
        match self.cache.entry(hash) {
            MapEntry::Vacant(e) => {
                let content_idx = self.content_pack.add_content(content)?;
                e.insert(content_idx);
                Ok(content_idx)
            }
            MapEntry::Occupied(e) => Ok(*e.get()),
        }
    }

    fn into_inner(self) -> jbk::creator::ContentPackCreator {
        self.content_pack
    }
}

pub struct Creator {
    content_pack: CachedContentPack,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store: Box<jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>>,
    dir_cache: DirEntry,
    strip_prefix: PathBuf,
    tmp_path_content_pack: tempfile::TempPath,
    tmp_path_directory_pack: tempfile::TempPath,
}

impl Creator {
    pub fn new<P: AsRef<Path>>(outfile: P, strip_prefix: PathBuf) -> jbk::Result<Self> {
        let outfile = outfile.as_ref();
        let out_dir = outfile.parent().unwrap();

        let (tmp_content_pack, tmp_path_content_pack) =
            tempfile::NamedTempFile::new_in(out_dir)?.into_parts();
        let content_pack = jbk::creator::ContentPackCreator::new_from_file(
            tmp_content_pack,
            jbk::PackId::from(1),
            VENDOR_ID,
            jbk::FreeData40::clone_from_slice(&[0x00; 40]),
            jbk::CompressionType::Zstd,
        )?;

        let (_, tmp_path_directory_pack) = tempfile::NamedTempFile::new_in(out_dir)?.into_parts();
        let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
            &tmp_path_directory_pack,
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

        let root_entry = DirEntry::new_root();

        Ok(Self {
            content_pack: CachedContentPack::new(content_pack),
            directory_pack,
            entry_store,
            dir_cache: root_entry,
            strip_prefix,
            tmp_path_content_pack,
            tmp_path_directory_pack,
        })
    }

    pub fn finalize(mut self, outfile: PathBuf) -> Void {
        let entry_count = self.entry_store.len();
        let entry_store_id = self.directory_pack.add_entry_store(self.entry_store);
        self.directory_pack.create_index(
            "arx_entries",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(entry_count as u32),
            jubako::EntryIdx::from(0).into(),
        );
        self.directory_pack.create_index(
            "arx_root",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(self.dir_cache.entry_count_generator()() as u32),
            jubako::EntryIdx::from(0).into(),
        );
        let mut outfilename = outfile.file_name().unwrap().to_os_string();
        outfilename.push(".jbkd");
        let mut directory_pack_path = PathBuf::new();
        directory_pack_path.push(&outfile);
        directory_pack_path.set_file_name(outfilename);
        let directory_pack_info = self
            .directory_pack
            .finalize(Some(directory_pack_path.clone()))?;
        if let Err(e) = self.tmp_path_directory_pack.persist(&directory_pack_path) {
            return Err(e.error.into());
        };
        let mut outfilename = outfile.file_name().unwrap().to_os_string();
        outfilename.push(".jbkc");
        let mut content_pack_path = PathBuf::new();
        content_pack_path.push(&outfile);
        content_pack_path.set_file_name(outfilename);
        let content_pack_info = self
            .content_pack
            .into_inner()
            .finalize(Some(content_pack_path.clone()))?;
        if let Err(e) = self.tmp_path_content_pack.persist(&content_pack_path) {
            return Err(e.error.into());
        }

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

    pub fn add_from_path<P: AsRef<std::path::Path>>(&mut self, path: P, recurse: bool) -> Void {
        self.add_from_path_with_filter(path, recurse, &Some)
    }

    pub fn add_from_path_with_filter<P, F>(&mut self, path: P, recurse: bool, filter: &F) -> Void
    where
        P: AsRef<std::path::Path>,
        F: Fn(Entry) -> Option<Entry>,
    {
        let rel_path = path.as_ref().strip_prefix(&self.strip_prefix).unwrap();
        let dir_cache: &mut DirEntry = if let Some(parents) = rel_path.parent() {
            self.dir_cache.mk_dirs(
                self.strip_prefix.clone(),
                parents.components(),
                &mut self.entry_store,
            )?
        } else {
            &mut self.dir_cache
        };
        if rel_path.as_os_str().is_empty() {
            for sub_entry in fs::read_dir(path)? {
                let sub_entry = sub_entry?;
                dir_cache.add(
                    &sub_entry.path(),
                    &sub_entry.file_name(),
                    recurse,
                    filter,
                    &mut self.entry_store,
                    &mut |r| self.content_pack.add_content(r),
                )?;
            }
            Ok(())
        } else {
            dir_cache.add(
                path.as_ref(),
                path.as_ref().file_name().unwrap(),
                recurse,
                filter,
                &mut self.entry_store,
                &mut |r| self.content_pack.add_content(r),
            )
        }
    }
}
