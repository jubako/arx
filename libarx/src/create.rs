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
use std::sync::Arc;

const VENDOR_ID: u32 = 0x41_52_58_00;

pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

pub enum EntryKind {
    Dir(Box<dyn Iterator<Item = jbk::Result<Box<dyn EntryTrait>>>>),
    File(jbk::Reader),
    Link(OsString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jbk::Result<EntryKind>;

    /// Under which name the entry will be stored
    fn name(&self) -> &OsStr;

    fn uid(&self) -> u64;
    fn gid(&self) -> u64;
    fn mode(&self) -> u64;
    fn mtime(&self) -> u64;
}

impl<T> EntryTrait for Box<T>
where
    T: EntryTrait + ?Sized,
{
    fn kind(&self) -> jbk::Result<EntryKind> {
        self.as_ref().kind()
    }
    fn name(&self) -> &OsStr {
        self.as_ref().name()
    }

    fn uid(&self) -> u64 {
        self.as_ref().uid()
    }
    fn gid(&self) -> u64 {
        self.as_ref().gid()
    }
    fn mode(&self) -> u64 {
        self.as_ref().mode()
    }
    fn mtime(&self) -> u64 {
        self.as_ref().mtime()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum FsEntryKind {
    Dir,
    File,
    Link,
    Other,
}

type Filter = Rc<dyn Fn(FsEntry) -> Option<FsEntry>>;

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub path: PathBuf,
    pub name: OsString,
    recurse: bool,
    filter: Filter,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl FsEntry {
    fn new(path: PathBuf, name: OsString, recurse: bool, filter: Filter) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: FsEntryKind::Dir,
                path,
                name,
                recurse,
                filter,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else if attr.is_file() {
            Self {
                kind: FsEntryKind::File,
                path,
                name,
                recurse,
                filter,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else if attr.is_symlink() {
            Self {
                kind: FsEntryKind::Link,
                path,
                name,
                recurse,
                filter,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        } else {
            Self {
                kind: FsEntryKind::Other,
                path,
                name,
                recurse,
                filter,
                uid: attr.uid() as u64,
                gid: attr.gid() as u64,
                mode: attr.mode() as u64,
                mtime: attr.mtime() as u64,
            }
        })
    }

    pub fn new_from_fs(
        dir_entry: fs::DirEntry,
        recurse: bool,
        filter: Filter,
    ) -> jbk::Result<Self> {
        let path = dir_entry.path();
        let name = dir_entry.file_name();
        Ok(if let Ok(file_type) = dir_entry.file_type() {
            let attr = fs::symlink_metadata(&path)?;
            if file_type.is_dir() {
                Self {
                    kind: FsEntryKind::Dir,
                    path,
                    name,
                    recurse,
                    filter,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else if file_type.is_file() {
                Self {
                    kind: FsEntryKind::File,
                    path,
                    name,
                    recurse,
                    filter,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: FsEntryKind::Link,
                    path,
                    name,
                    recurse,
                    filter,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            } else {
                Self {
                    kind: FsEntryKind::Other,
                    path,
                    name,
                    recurse,
                    filter,
                    uid: attr.uid() as u64,
                    gid: attr.gid() as u64,
                    mode: attr.mode() as u64,
                    mtime: attr.mtime() as u64,
                }
            }
        } else {
            Self {
                kind: FsEntryKind::Other,
                path,
                name,
                recurse,
                filter,
                uid: 0,
                gid: 0,
                mode: 0,
                mtime: 0,
            }
        })
    }
}

impl EntryTrait for FsEntry {
    fn kind(&self) -> jbk::Result<EntryKind> {
        Ok(match self.kind {
            FsEntryKind::Dir => {
                let filter = Rc::clone(&self.filter);
                let recurse = self.recurse;
                EntryKind::Dir(Box::new(fs::read_dir(self.path.clone())?.map(
                    move |dir_entry| {
                        Ok(Box::new(FsEntry::new_from_fs(
                            dir_entry?,
                            recurse,
                            Rc::clone(&filter),
                        )?) as Box<dyn EntryTrait + 'static>)
                    },
                )))
            }
            FsEntryKind::File => {
                EntryKind::File(jbk::creator::FileSource::open(&self.path)?.into())
            }
            FsEntryKind::Link => EntryKind::Link(fs::read_link(&self.path)?.into()),
            FsEntryKind::Other => unreachable!(),
        })
    }
    fn name(&self) -> &OsStr {
        &self.name
    }

    fn uid(&self) -> u64 {
        self.uid
    }
    fn gid(&self) -> u64 {
        self.gid
    }
    fn mode(&self) -> u64 {
        self.mode
    }
    fn mtime(&self) -> u64 {
        self.mtime
    }
}

struct SimpleDir {
    path: PathBuf,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl SimpleDir {
    fn new(path: PathBuf) -> Self {
        let attr = fs::symlink_metadata(&path).unwrap();
        Self {
            path,
            uid: attr.uid() as u64,
            gid: attr.gid() as u64,
            mode: attr.mode() as u64,
            mtime: attr.mtime() as u64,
        }
    }
}

impl EntryTrait for SimpleDir {
    fn kind(&self) -> jbk::Result<EntryKind> {
        Ok(EntryKind::Dir(Box::new(std::iter::empty())))
    }
    fn name(&self) -> &OsStr {
        self.path.file_name().unwrap()
    }

    fn uid(&self) -> u64 {
        self.uid
    }
    fn gid(&self) -> u64 {
        self.gid
    }
    fn mode(&self) -> u64 {
        self.mode
    }
    fn mtime(&self) -> u64 {
        self.mtime
    }
}

type DirCache = HashMap<OsString, DirEntry>;
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

    fn mk_dirs(
        &mut self,
        mut path: PathBuf,
        mut components: std::path::Components,
        entry_store: &mut jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>,
    ) -> jbk::Result<&mut Self> {
        if let Some(component) = components.next() {
            path.push(component);
            let entry = SimpleDir::new(path.clone());
            if !self
                .dir_children
                .contains_key::<OsStr>(component.as_os_str())
            {
                self.add(entry, entry_store, &mut |_| unreachable!())?;
            }
            unsafe { Rc::get_mut_unchecked(&mut self.dir_children) }
                .get_mut::<OsStr>(component.as_os_str())
                .unwrap()
                .mk_dirs(path, components, entry_store)
        } else {
            Ok(self)
        }
    }

    fn add<E, Adder>(
        &mut self,
        entry: E,
        entry_store: &mut jbk::creator::EntryStore<Box<jbk::creator::BasicEntry>>,
        add_content: &mut Adder,
    ) -> Void
    where
        E: EntryTrait,
        Adder: FnMut(jbk::Reader) -> jbk::Result<jbk::ContentIdx>,
    {
        match entry.kind()? {
            EntryKind::Dir(children) => {
                if self.dir_children.contains_key(entry.name()) {
                    return Ok(());
                }
                let entry_idx = jbk::Vow::new(jbk::EntryIdx::from(0));
                let mut dir_entry = DirEntry::new(entry_idx.bind());

                {
                    let entry = Box::new(jbk::creator::BasicEntry::new_from_schema_idx(
                        &entry_store.schema,
                        entry_idx,
                        Some(EntryType::Dir.into()),
                        vec![
                            jbk::Value::Array(entry.name().to_os_string().into_vec()),
                            jbk::Value::Unsigned(self.as_parent_idx_generator().into()),
                            jbk::Value::Unsigned(entry.uid().into()),
                            jbk::Value::Unsigned(entry.gid().into()),
                            jbk::Value::Unsigned(entry.mode().into()),
                            jbk::Value::Unsigned(entry.mtime().into()),
                            jbk::Value::Unsigned(dir_entry.first_entry_generator().into()),
                            jbk::Value::Unsigned(dir_entry.entry_count_generator().into()),
                        ],
                    ));
                    entry_store.add_entry(entry);
                }
                for sub_entry in children {
                    dir_entry.add(sub_entry?, entry_store, add_content)?;
                }

                /* SAFETY: We already have Rc on `self.dir_children` but it is only used
                  in a second step to get entry_count and min entry_idx.
                  So while we borrow `self.dir_children` we never read it otherwise.
                */
                unsafe { Rc::get_mut_unchecked(&mut self.dir_children) }
                    .entry(entry.name().into())
                    .or_insert(dir_entry);
                Ok(())
            }
            EntryKind::File(reader) => {
                let size = reader.size();
                let content_id = add_content(reader)?;
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::File.into()),
                    vec![
                        jbk::Value::Array(entry.name().to_os_string().into_vec()),
                        jbk::Value::Unsigned(self.as_parent_idx_generator().into()),
                        jbk::Value::Unsigned(entry.uid().into()),
                        jbk::Value::Unsigned(entry.gid().into()),
                        jbk::Value::Unsigned(entry.mode().into()),
                        jbk::Value::Unsigned(entry.mtime().into()),
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
            EntryKind::Link(target) => {
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::Link.into()),
                    vec![
                        jbk::Value::Array(entry.name().to_os_string().into_vec()),
                        jbk::Value::Unsigned(self.as_parent_idx_generator().into()),
                        jbk::Value::Unsigned(entry.uid().into()),
                        jbk::Value::Unsigned(entry.gid().into()),
                        jbk::Value::Unsigned(entry.mode().into()),
                        jbk::Value::Unsigned(entry.mtime().into()),
                        jbk::Value::Array(target.into_vec()),
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
    concat_mode: ConcatMode,
    tmp_path_content_pack: tempfile::TempPath,
    tmp_path_directory_pack: tempfile::TempPath,
}

impl Creator {
    pub fn new<P: AsRef<Path>>(
        outfile: P,
        strip_prefix: PathBuf,
        concat_mode: ConcatMode,
        progress: Arc<dyn jbk::creator::Progress>,
    ) -> jbk::Result<Self> {
        let outfile = outfile.as_ref();
        let out_dir = outfile.parent().unwrap();

        let (tmp_content_pack, tmp_path_content_pack) =
            tempfile::NamedTempFile::new_in(out_dir)?.into_parts();
        let content_pack = jbk::creator::ContentPackCreator::new_from_file_with_progress(
            tmp_content_pack,
            jbk::PackId::from(1),
            VENDOR_ID,
            jbk::FreeData40::clone_from_slice(&[0x00; 40]),
            jbk::CompressionType::Zstd,
            progress,
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
            concat_mode,
            tmp_path_content_pack,
            tmp_path_directory_pack,
        })
    }

    pub fn finalize(mut self, outfile: &Path) -> Void {
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

        let directory_pack_info = match self.concat_mode {
            ConcatMode::NoConcat => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".jbkd");
                let mut directory_pack_path = PathBuf::new();
                directory_pack_path.push(outfile);
                directory_pack_path.set_file_name(outfilename);
                let directory_pack_info = self
                    .directory_pack
                    .finalize(Some(directory_pack_path.clone()))?;
                if let Err(e) = self.tmp_path_directory_pack.persist(&directory_pack_path) {
                    return Err(e.error.into());
                };
                directory_pack_info
            }
            _ => self.directory_pack.finalize(None)?,
        };

        let content_pack_info = match self.concat_mode {
            ConcatMode::OneFile => self.content_pack.into_inner().finalize(None)?,
            _ => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".jbkc");
                let mut content_pack_path = PathBuf::new();
                content_pack_path.push(outfile);
                content_pack_path.set_file_name(outfilename);
                let content_pack_info = self
                    .content_pack
                    .into_inner()
                    .finalize(Some(content_pack_path.clone()))?;
                if let Err(e) = self.tmp_path_content_pack.persist(&content_pack_path) {
                    return Err(e.error.into());
                }
                content_pack_info
            }
        };

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
        self.add_from_path_with_filter(path, recurse, Rc::new(&Some))
    }

    pub fn add_from_path_with_filter<P>(&mut self, path: P, recurse: bool, filter: Filter) -> Void
    where
        P: AsRef<std::path::Path>,
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
            if recurse {
                for sub_entry in fs::read_dir(path)? {
                    let sub_entry = sub_entry?;
                    dir_cache.add(
                        FsEntry::new_from_fs(sub_entry, recurse, Rc::clone(&filter))?,
                        &mut self.entry_store,
                        &mut |r| self.content_pack.add_content(r),
                    )?;
                }
            }
            Ok(())
        } else {
            dir_cache.add(
                FsEntry::new(
                    path.as_ref().to_path_buf(),
                    path.as_ref().file_name().unwrap().to_os_string(),
                    recurse,
                    filter,
                )?,
                &mut self.entry_store,
                &mut |r| self.content_pack.add_content(r),
            )
        }
    }

    pub fn add_entry<E>(&mut self, entry: E) -> Void
    where
        E: EntryTrait,
    {
        self.dir_cache.add(entry, &mut self.entry_store, &mut |r| {
            self.content_pack.add_content(r)
        })
    }
}
