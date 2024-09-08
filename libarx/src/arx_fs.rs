use super::Arx;
use crate::common::{AllProperties, Comparator, EntryType, ReadEntry};
use fxhash::FxBuildHasher;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::{MayMissPack, Range};
use jbk::EntryRange;
use libc::ENOENT;
use lru::LruCache;
use std::cmp::min;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

pub type EntryResult<T> = Result<T, EntryType>;
const TTL: std::time::Duration = std::time::Duration::from_secs(1000); // Nothing change on oar side, TTL is long
const BLOCK_SIZE: u32 = 512;

pub trait Stats {
    fn lookup(&mut self) {}
    fn getattr(&mut self) {}
    fn readlink(&mut self) {}
    fn open(&mut self) {}
    fn read(&mut self) {}
    fn release(&mut self) {}
    fn opendir(&mut self) {}
    fn readdir(&mut self) {}
    fn releasedir(&mut self) {}
}

impl Stats for () {}

// Root ino (from kernel pov) is 1.
// However, our root is the "root" index and it doesn't have a ino
// and the first entry we have (a child of "root" index) is 0
// So we do the following mapping :
// - ino 0 (kernel) => invalid
// - ino 1 (kernel) => "root" index
// - ino x>=2 (kernel) => entry x-2
// On the opposite side:
// - entry n => send inode n+2

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct Ino(NonZeroU64);

impl Ino {
    fn get(&self) -> u64 {
        self.0.get()
    }
}

impl From<u64> for Ino {
    fn from(v: u64) -> Self {
        assert_ne!(v, 0);
        Self(unsafe { NonZeroU64::new_unchecked(v) })
    }
}

impl From<jbk::EntryIdx> for Ino {
    fn from(idx: jbk::EntryIdx) -> Ino {
        Self::from(idx.into_u64() + 2)
    }
}

impl TryInto<jbk::EntryIdx> for Ino {
    type Error = ();

    fn try_into(self) -> Result<jbk::EntryIdx, ()> {
        match self.0.get() {
            1 => Err(()),
            v => Ok(jbk::EntryIdx::from((v - 2) as u32)),
        }
    }
}

struct LightLinkBuilder {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    link_property: jbk::reader::builder::ArrayProperty,
}

impl LightLinkBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            link_property: properties.link_target_property.clone(),
        }
    }
}

impl jbk::reader::builder::BuilderTrait for LightLinkBuilder {
    type Entry = EntryResult<Vec<u8>>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        Ok(
            match self.variant_id_property.create(&reader)?.try_into()? {
                EntryType::Link => {
                    let target = self.link_property.create(&reader)?;
                    let mut vec = vec![];
                    target.resolve_to_vec(&mut vec)?;
                    Ok(vec)
                }
                other => Err(other),
            },
        )
    }
}

struct LightFileBuilder {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl LightFileBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            content_address_property: properties.file_content_address_property,
        }
    }
}

impl jbk::reader::builder::BuilderTrait for LightFileBuilder {
    type Entry = EntryResult<jbk::reader::ContentAddress>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        Ok(
            match self.variant_id_property.create(&reader)?.try_into()? {
                EntryType::File => {
                    let content_address = self.content_address_property.create(&reader)?;
                    Ok(content_address)
                }
                other => Err(other),
            },
        )
    }
}

struct LightDirBuilder {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
}

impl LightDirBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
        }
    }
}

impl jbk::reader::builder::BuilderTrait for LightDirBuilder {
    type Entry = EntryResult<jbk::EntryRange>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        Ok(
            match self.variant_id_property.create(&reader)?.try_into()? {
                EntryType::Dir => {
                    let first_child: jbk::EntryIdx =
                        (self.first_child_property.create(&reader)? as u32).into();
                    let nb_children: jbk::EntryCount =
                        (self.nb_children_property.create(&reader)? as u32).into();
                    Ok(jbk::EntryRange::new_from_size(first_child, nb_children))
                }
                other => Err(other),
            },
        )
    }
}

struct LightCommonPath {
    file_type: EntryType,
    path: Vec<u8>,
}

struct LightCommonPathBuilder {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    path_property: jbk::reader::builder::ArrayProperty,
}

impl LightCommonPathBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            path_property: properties.path_property.clone(),
        }
    }
}

impl jbk::reader::builder::BuilderTrait for LightCommonPathBuilder {
    type Entry = LightCommonPath;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let path_prop = self.path_property.create(&reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        let file_type = self.variant_id_property.create(&reader)?.try_into()?;
        Ok(LightCommonPath { file_type, path })
    }
}

struct LightCommonParentBuilder {
    store: jbk::reader::EntryStore,
    parent_property: jbk::reader::builder::IntProperty,
}

impl LightCommonParentBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            parent_property: properties.parent_property.clone(),
        }
    }
}

impl jbk::reader::builder::BuilderTrait for LightCommonParentBuilder {
    type Entry = Option<jbk::EntryIdx>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let parent = self.parent_property.create(&reader)?;
        let parent = if parent == 0 {
            None
        } else {
            Some((parent as u32 - 1).into())
        };
        Ok(parent)
    }
}

struct AttrBuilder {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    owner_property: jbk::reader::builder::IntProperty,
    group_property: jbk::reader::builder::IntProperty,
    rights_property: jbk::reader::builder::IntProperty,
    mtime_property: jbk::reader::builder::IntProperty,
    file_size_property: jbk::reader::builder::IntProperty,
    dir_nb_children_property: jbk::reader::builder::IntProperty,
    link_target_property: jbk::reader::builder::ArrayProperty,
}

impl AttrBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            owner_property: properties.owner_property.clone(),
            group_property: properties.group_property.clone(),
            rights_property: properties.rigths_property.clone(),
            mtime_property: properties.mtime_property.clone(),
            file_size_property: properties.file_size_property.clone(),
            dir_nb_children_property: properties.dir_nb_children_property.clone(),
            link_target_property: properties.link_target_property.clone(),
        }
    }
}

fn div_ceil(value: u64, rhs: u64) -> u64 {
    let mut ret = value / rhs;
    if (value % rhs) != 0 {
        ret += 1;
    }
    ret
}

impl jbk::reader::builder::BuilderTrait for AttrBuilder {
    type Entry = fuser::FileAttr;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let kind = self.variant_id_property.create(&reader)?.try_into()?;

        let size = match &kind {
            EntryType::File => self.file_size_property.create(&reader)?,
            EntryType::Dir => (self.dir_nb_children_property.create(&reader)? + 1) * 10,
            EntryType::Link => {
                let link = self.link_target_property.create(&reader)?;
                match link.size() {
                    Some(s) => s as u64,
                    None => {
                        let mut vec = vec![];
                        link.resolve_to_vec(&mut vec)?;
                        vec.len() as u64
                    }
                }
            }
        };
        let rigths = (self.rights_property.create(&reader)? as u16) & 0b1111_1111_0110_1101;
        // Make kernel sync we allocate by block of 4KB.
        let allocated_size = match &kind {
            EntryType::Dir => 0,
            _ => div_ceil(size, 4 * 1024) * (4 * 1024),
        };
        Ok(fuser::FileAttr {
            ino: Ino::from(idx).get(),
            size,
            kind: kind.into(),
            blocks: div_ceil(allocated_size, BLOCK_SIZE as u64),
            atime: std::time::UNIX_EPOCH,
            mtime: std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(self.mtime_property.create(&reader)?),
            ctime: std::time::UNIX_EPOCH,
            crtime: std::time::UNIX_EPOCH,
            perm: rigths,
            nlink: 1,
            uid: self.owner_property.create(&reader)? as u32,
            gid: self.group_property.create(&reader)? as u32,
            rdev: 0,
            blksize: BLOCK_SIZE,
            flags: 0,
        })
    }
}

static mut NOSTATS: () = ();

pub struct ArxFs<'a, S: Stats> {
    arx: Arx,
    entry_index: jbk::reader::Index,
    root_range: EntryRange,
    comparator: Comparator,
    light_file_builder: LightFileBuilder,
    light_dir_builder: LightDirBuilder,
    light_link_builder: LightLinkBuilder,
    light_common_path_builder: LightCommonPathBuilder,
    light_common_parent_builder: LightCommonParentBuilder,
    attr_builder: AttrBuilder,
    resolve_cache: LruCache<(Ino, OsString), Option<jbk::EntryIdx>, FxBuildHasher>,
    attr_cache: LruCache<jbk::EntryIdx, fuser::FileAttr, FxBuildHasher>,
    region_cache: HashMap<Ino, (jbk::reader::ByteRegion, u64), FxBuildHasher>,
    stats: &'a mut S,
}

impl ArxFs<'static, ()> {
    pub fn new(arx: Arx) -> jbk::Result<Self> {
        // SAFETY: No data race can occurs on empty type doing nothing
        let root_range = (&arx.root_index).into();
        Self::new_from_root(arx, root_range)
    }
    pub fn new_from_root(arx: Arx, root_range: EntryRange) -> jbk::Result<Self> {
        // SAFETY: No data race can occurs on empty type doing nothing
        Self::new_with_stats(arx, root_range, unsafe {
            &mut *std::ptr::addr_of_mut!(NOSTATS)
        })
    }
}

impl<'a, S: Stats> ArxFs<'a, S> {
    pub fn new_with_stats(arx: Arx, root_range: EntryRange, stats: &'a mut S) -> jbk::Result<Self> {
        let entry_index = arx.get_index_for_name("arx_entries")?;
        let properties = arx.create_properties(&entry_index)?;
        let comparator = Comparator::new(&properties);
        let light_file_builder = LightFileBuilder::new(&properties);
        let light_dir_builder = LightDirBuilder::new(&properties);
        let light_link_builder = LightLinkBuilder::new(&properties);
        let light_common_path_builder = LightCommonPathBuilder::new(&properties);
        let light_common_parent_builder = LightCommonParentBuilder::new(&properties);
        let attr_builder = AttrBuilder::new(&properties);
        Ok(Self {
            arx,
            entry_index,
            root_range,
            comparator,
            light_file_builder,
            light_dir_builder,
            light_link_builder,
            light_common_path_builder,
            light_common_parent_builder,
            attr_builder,
            resolve_cache: LruCache::with_hasher(
                NonZeroUsize::new(4 * 1024).unwrap(),
                FxBuildHasher::default(),
            ),
            attr_cache: LruCache::with_hasher(
                NonZeroUsize::new(100).unwrap(),
                FxBuildHasher::default(),
            ),
            region_cache: HashMap::with_hasher(FxBuildHasher::default()),
            stats,
        })
    }

    fn get_entry_range(&self, ino: Ino) -> jbk::Result<jbk::EntryRange> {
        match ino.try_into() {
            Err(_) => Ok(self.root_range),
            Ok(idx) => match self.entry_index.get_entry(&self.light_dir_builder, idx)? {
                Ok(r) => Ok(r),
                Err(_) => Err("No at directory".to_string().into()),
            },
        }
    }

    fn mount_options(&self, name: String) -> Vec<fuser::MountOption> {
        vec![
            fuser::MountOption::RO,
            fuser::MountOption::FSName(name),
            fuser::MountOption::Subtype("arx".into()),
            fuser::MountOption::DefaultPermissions,
        ]
    }

    pub fn mount<P: AsRef<Path>>(self, name: String, mount_point: P) -> jbk::Result<()> {
        let options = self.mount_options(name);
        fuser::mount2(self, &mount_point, &options)?;
        Ok(())
    }
}

impl<S: Stats + Send> ArxFs<'static, S> {
    pub fn spawn_mount<P: AsRef<Path>>(
        self,
        name: String,
        mount_point: P,
    ) -> jbk::Result<fuser::BackgroundSession> {
        let options = self.mount_options(name);
        Ok(fuser::spawn_mount2(self, &mount_point, &options)?)
    }
}

const ROOT_ATTR: fuser::FileAttr = fuser::FileAttr {
    ino: 1,
    size: 0,
    kind: fuser::FileType::Directory,
    blocks: 1,
    atime: std::time::UNIX_EPOCH,
    mtime: std::time::UNIX_EPOCH,
    ctime: std::time::UNIX_EPOCH,
    crtime: std::time::UNIX_EPOCH,
    perm: 0o555,
    nlink: 2,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    blksize: 0,
    flags: 0,
};

impl<'a, S: Stats> fuser::Filesystem for ArxFs<'a, S> {
    fn lookup(
        &mut self,
        _req: &fuser::Request,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEntry,
    ) {
        self.stats.lookup();
        let parent = Ino::from(parent);
        // Lookup for entry `name` in directory `parent`
        // First get parent finder
        let idx = self.resolve_cache.get(&(parent, name.to_os_string()));
        let idx = match idx {
            Some(idx) => *idx,
            None => {
                let range = self.get_entry_range(parent).unwrap();
                let comparator = self.comparator.compare_with(name.as_bytes());
                let idx = range
                    .find(&comparator)
                    .unwrap()
                    .map(|idx| idx + range.offset());
                self.resolve_cache.put((parent, name.to_os_string()), idx);
                idx
            }
        };
        match idx {
            None => reply.error(ENOENT),
            Some(idx) => {
                let attr = self.attr_cache.get(&idx);
                let attr = match attr {
                    Some(attr) => attr,
                    None => {
                        let attr = self.entry_index.get_entry(&self.attr_builder, idx).unwrap();
                        self.attr_cache.push(idx, attr);
                        self.attr_cache.get(&idx).unwrap()
                    }
                };
                reply.entry(&TTL, attr, 0)
            }
        }
    }

    fn getattr(&mut self, _req: &fuser::Request, ino: u64, reply: fuser::ReplyAttr) {
        self.stats.getattr();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => {
                reply.attr(&TTL, &ROOT_ATTR);
            }
            Ok(idx) => {
                let attr = self.attr_cache.get(&idx);
                let attr = match attr {
                    Some(attr) => attr,
                    None => {
                        let attr = self.entry_index.get_entry(&self.attr_builder, idx).unwrap();
                        self.attr_cache.push(idx, attr);
                        self.attr_cache.get(&idx).unwrap()
                    }
                };
                reply.attr(&TTL, attr);
            }
        }
    }

    fn readlink(&mut self, _req: &fuser::Request, ino: u64, reply: fuser::ReplyData) {
        self.stats.readlink();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.error(libc::ENOLINK),
            Ok(idx) => {
                let entry = self
                    .entry_index
                    .get_entry(&self.light_link_builder, idx)
                    .unwrap();
                match &entry {
                    Ok(link) => reply.data(link),
                    Err(_) => reply.error(libc::ENOLINK),
                }
            }
        }
    }

    fn open(&mut self, _req: &fuser::Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        self.stats.open();
        let ino = Ino::from(ino);
        match self.region_cache.get_mut(&ino) {
            Some((_r, c)) => {
                *c += 1;
                reply.opened(0, fuser::consts::FOPEN_KEEP_CACHE);
            }
            None => match ino.try_into() {
                Err(_) => reply.error(libc::EISDIR),
                Ok(idx) => {
                    let entry = self
                        .entry_index
                        .get_entry(&self.light_file_builder, idx)
                        .unwrap();
                    match &entry {
                        Ok(content_address) => match self.arx.get_bytes(*content_address) {
                            Err(_e) => reply.error(libc::EIO),
                            Ok(MayMissPack::MISSING(_pack_info)) => reply.error(
                                #[cfg(not(target_os = "linux"))]
                                libc::ENODATA,
                                #[cfg(target_os = "linux")]
                                libc::ENOMEDIUM,
                            ),
                            Ok(MayMissPack::FOUND(bytes)) => {
                                self.region_cache.insert(ino, (bytes, 1));
                                reply.opened(0, fuser::consts::FOPEN_KEEP_CACHE);
                            }
                        },
                        Err(EntryType::Dir) => reply.error(libc::EISDIR),
                        Err(EntryType::Link) => reply.error(libc::ENOENT), // [FIXME] What to return here ?
                        Err(EntryType::File) => unreachable!(),
                    }
                }
            },
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        self.stats.read();
        let ino = Ino::from(ino);
        let offset: u64 = offset.try_into().unwrap();
        let region = &self.region_cache.get(&ino).unwrap().0;
        let size = min(size as u64, region.size().into_u64() - offset) as usize;
        let data = region.get_slice(offset.into(), size).unwrap();
        reply.data(&data)
    }

    fn release(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.stats.release();
        let ino = Ino::from(ino);
        match self.region_cache.get_mut(&ino) {
            Some((_r, c)) => {
                *c -= 1;
                if *c == 0 {
                    self.region_cache.remove(&ino);
                }
                reply.ok()
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn opendir(&mut self, _req: &fuser::Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        self.stats.opendir();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.opened(0, fuser::consts::FOPEN_KEEP_CACHE),
            Ok(idx) => {
                let entry = self
                    .entry_index
                    .get_entry(&self.light_dir_builder, idx)
                    .unwrap();
                match &entry {
                    Ok(_) => reply.opened(0, fuser::consts::FOPEN_KEEP_CACHE),
                    Err(_) => reply.error(libc::ENOTDIR),
                }
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        self.stats.readdir();
        let ino = Ino::from(ino);
        let range = self.get_entry_range(ino).unwrap();
        let nb_entry = (range.count().into_u32() + 2) as i64; // we include "." and ".."
        let mut readentry = ReadEntry::new(&range, &self.light_common_path_builder);
        // If offset != 0, offset corresponds to what has already been seen. So we must start after.
        let offset = if offset == 0 { 0 } else { offset + 1 };
        if offset > 2 {
            // We skip offset entries (minus "." and "..")
            ReadEntry::skip(&mut readentry, jbk::EntryCount::from((offset - 2) as u32));
        }
        for i in offset..nb_entry {
            if i == 0 {
                if reply.add(ino.get(), i, fuser::FileType::Directory, ".") {
                    break;
                }
            } else if i == 1 {
                let parent_ino = match ino.try_into() {
                    Err(_) => ino,
                    Ok(idx) => {
                        let parent = self
                            .entry_index
                            .get_entry(&self.light_common_parent_builder, idx)
                            .unwrap();
                        match parent {
                            None => Ino::from(1),
                            Some(parent_id) => parent_id.into(),
                        }
                    }
                };
                if reply.add(parent_ino.get(), i, fuser::FileType::Directory, "..") {
                    break;
                }
            } else {
                match readentry.next() {
                    None => break,
                    Some(entry) => {
                        let entry = entry.unwrap();
                        // We remove "." and ".."
                        let entry_idx = range.offset() + jbk::EntryIdx::from(i as u32 - 2);
                        let entry_ino = Ino::from(entry_idx);
                        let entry_path = OsString::from_vec(entry.path);
                        let should_break = reply.add(
                            entry_ino.get(),
                            /* offset =*/ i,
                            entry.file_type.into(),
                            &entry_path,
                        );
                        self.resolve_cache.put((ino, entry_path), Some(entry_idx));
                        if should_break {
                            break;
                        }
                    }
                }
            }
        }
        reply.ok();
    }

    fn releasedir(
        &mut self,
        _req: &fuser::Request,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        self.stats.releasedir();
        reply.ok()
    }
}
