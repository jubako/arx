use crate::common::{Arx, Entry, EntryKind, ReadEntry};
use jubako as jbk;
//use jbk::reader::Finder;
use libc::ENOENT;
use lru::LruCache;
use std::cmp::min;
use std::ffi::{OsStr, OsString};
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

const TTL: time::Timespec = time::Timespec { sec: 1000, nsec: 0 }; // Nothing change on oar side, TTL is long
const UNIX_EPOCH: time::Timespec = time::Timespec { sec: 0, nsec: 0 };

struct StatCounter {
    nb_lookup: u64,
    nb_getattr: u64,
    nb_readlink: u64,
    nb_open: u64,
    nb_read: u64,
    nb_release: u64,
    nb_opendir: u64,
    nb_readdir: u64,
    nb_releasedir: u64,
}

impl StatCounter {
    pub fn new() -> Self {
        Self {
            nb_lookup: 0,
            nb_getattr: 0,
            nb_readlink: 0,
            nb_open: 0,
            nb_read: 0,
            nb_release: 0,
            nb_opendir: 0,
            nb_readdir: 0,
            nb_releasedir: 0,
        }
    }

    pub fn lookup(&mut self) {
        self.nb_lookup += 1;
    }

    pub fn getattr(&mut self) {
        self.nb_getattr += 1;
    }

    pub fn readlink(&mut self) {
        self.nb_readlink += 1;
    }

    pub fn open(&mut self) {
        self.nb_open += 1;
    }

    pub fn read(&mut self) {
        self.nb_read += 1;
    }

    pub fn release(&mut self) {
        self.nb_release += 1;
    }

    pub fn opendir(&mut self) {
        self.nb_opendir += 1;
    }

    pub fn readdir(&mut self) {
        self.nb_readdir += 1;
    }

    pub fn releasedir(&mut self) {
        self.nb_releasedir += 1;
    }
}

impl std::fmt::Display for StatCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "nb_lookup: {}", self.nb_lookup)?;
        writeln!(f, "nb_getattr: {}", self.nb_getattr)?;
        writeln!(f, "nb_readlink: {}", self.nb_readlink)?;
        writeln!(f, "nb_open: {}", self.nb_open)?;
        writeln!(f, "nb_read: {}", self.nb_read)?;
        writeln!(f, "nb_release: {}", self.nb_release)?;
        writeln!(f, "nb_opendir: {}", self.nb_opendir)?;
        writeln!(f, "nb_readdir: {}", self.nb_readdir)?;
        writeln!(f, "nb_releasedir: {}", self.nb_releasedir)?;
        Ok(())
    }
}

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

struct ArxFs<'a> {
    arx: Arx,
    resolver: jbk::reader::Resolver,
    entry_index: jbk::reader::Index,
    resolve_cache: LruCache<(Ino, OsString), Option<jbk::EntryIdx>>,
    attr_cache: LruCache<jbk::EntryIdx, fuse::FileAttr>,
    pub stats: &'a mut StatCounter,
}

impl<'a> ArxFs<'a> {
    pub fn new(arx: Arx, stats: &'a mut StatCounter) -> jbk::Result<Self> {
        let resolver = jbk::reader::Resolver::new(Rc::clone(arx.get_value_storage()));
        let entry_index = arx.get_index_for_name("entries")?;
        Ok(Self {
            arx,
            resolver,
            entry_index,
            resolve_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            attr_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            stats,
        })
    }

    pub fn get_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Entry> {
        let resolver = jbk::reader::Resolver::new(Rc::clone(self.arx.get_value_storage()));
        let finder = self
            .entry_index
            .get_finder(self.arx.get_entry_storage(), resolver)?;
        let entry = finder.get_entry(idx)?;
        Ok(Entry::new(
            idx,
            entry,
            Rc::clone(self.arx.get_value_storage()),
        ))
    }

    pub fn get_finder(&self, ino: Ino) -> jbk::Result<jbk::reader::Finder> {
        match ino.try_into() {
            Err(_) => {
                let index = self.arx.get_index_for_name("root")?;
                Ok(index.get_finder(self.arx.get_entry_storage(), self.resolver.clone())?)
            }
            Ok(idx) => {
                let entry = self.get_entry(idx)?;
                if !entry.is_dir() {
                    Err("Invalid entry".to_string().into())
                } else {
                    let offset = entry.get_first_child();
                    let count = entry.get_nb_children();
                    let store = self
                        .arx
                        .get_entry_storage()
                        .get_entry_store(self.entry_index.get_store_id())?;
                    Ok(jbk::reader::Finder::new(
                        Rc::clone(store),
                        offset,
                        count,
                        jbk::reader::Resolver::new(Rc::clone(self.arx.get_value_storage())),
                    ))
                }
            }
        }
    }
}

impl Entry {
    fn to_fillattr(&self, container: &jbk::reader::Container) -> jbk::Result<fuse::FileAttr> {
        let (size, kind) = match &self.get_type() {
            EntryKind::Directory => (
                (self.get_nb_children() + 1).into_u64() * 10,
                fuse::FileType::Directory,
            ),
            EntryKind::File => {
                let content_address = self.get_content_address();
                let reader = container.get_reader(&content_address)?;
                let size = reader.size();
                (size.into_u64(), fuse::FileType::RegularFile)
            }
            EntryKind::Link => (
                self.get_target_link()?.len() as u64,
                fuse::FileType::Symlink,
            ),
        };
        Ok(fuse::FileAttr {
            ino: Ino::from(self.idx()).get(),
            size,
            kind,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            perm: 0o555,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        })
    }
}

impl<'a> fuse::Filesystem for ArxFs<'a> {
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        self.stats.lookup();
        let parent = Ino::from(parent);
        // Lookup for entry `name` in directory `parent`
        // First get parent finder
        let idx = self.resolve_cache.get(&(parent, name.to_os_string()));
        let idx = match idx {
            Some(idx) => *idx,
            None => {
                let finder = self.get_finder(parent).unwrap();
                let idx = finder
                    .find(
                        jbk::PropertyIdx::from(0),
                        jbk::reader::Value::Array(name.to_os_string().into_vec()),
                    )
                    .unwrap()
                    .map(|idx| idx + finder.offset());
                self.resolve_cache.push((parent, name.to_os_string()), idx);
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
                        let finder = self
                            .entry_index
                            .get_finder(
                                self.arx.get_entry_storage(),
                                jbk::reader::Resolver::new(Rc::clone(self.arx.get_value_storage())),
                            )
                            .unwrap();
                        let entry = finder.get_entry(idx).unwrap();
                        let entry = Entry::new(idx, entry, Rc::clone(self.arx.get_value_storage()));
                        let attr = entry.to_fillattr(&self.arx).unwrap();
                        self.attr_cache.push(idx, attr);
                        self.attr_cache.get(&idx).unwrap()
                    }
                };
                reply.entry(&TTL, attr, 0)
            }
        }
    }

    fn getattr(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        self.stats.getattr();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => {
                let attr = fuse::FileAttr {
                    ino: ino.get(),
                    size: 0,
                    kind: fuse::FileType::Directory,
                    blocks: 1,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    perm: 0o555,
                    nlink: 2,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                };
                reply.attr(&TTL, &attr);
            }
            Ok(idx) => {
                let attr = self.attr_cache.get(&idx);
                let attr = match attr {
                    Some(attr) => attr,
                    None => {
                        let entry = self.get_entry(idx).unwrap();
                        let attr = entry.to_fillattr(&self.arx).unwrap();
                        self.attr_cache.push(idx, attr);
                        self.attr_cache.get(&idx).unwrap()
                    }
                };
                reply.attr(&TTL, attr);
            }
        }
    }

    fn readlink(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyData) {
        self.stats.readlink();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.error(libc::ENOLINK),
            Ok(idx) => {
                let entry = self.get_entry(idx).unwrap();
                match &entry.get_type() {
                    EntryKind::Link => {
                        let target_link = entry.get_target_link().unwrap();
                        reply.data(target_link.as_bytes())
                    }
                    _ => reply.error(libc::ENOLINK),
                }
            }
        }
    }

    fn open(&mut self, _req: &fuse::Request, ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
        self.stats.open();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.error(libc::EISDIR),
            Ok(idx) => {
                let entry = self.get_entry(idx).unwrap();
                match &entry.get_type() {
                    EntryKind::File => reply.opened(0, 0),
                    EntryKind::Directory => reply.error(libc::EISDIR),
                    EntryKind::Link => reply.error(libc::ENOENT), // [FIXME] What to return here ?
                }
            }
        }
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        self.stats.read();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.error(libc::EISDIR),
            Ok(idx) => {
                let entry = self.get_entry(idx).unwrap();
                match &entry.get_type() {
                    EntryKind::File => {
                        let content_address = entry.get_content_address();
                        let reader = self.arx.get_reader(&content_address).unwrap();
                        let mut stream =
                            reader.create_stream_from(jbk::Offset::new(offset.try_into().unwrap()));
                        let size = min(size as u64, stream.size().into_u64());
                        let data = stream.read_vec(size as usize).unwrap();
                        reply.data(&data)
                    }
                    EntryKind::Directory => reply.error(libc::EISDIR),
                    EntryKind::Link => reply.error(libc::ENOENT), // [FIXME] What to return here ?
                }
            }
        }
    }

    fn release(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: fuse::ReplyEmpty,
    ) {
        self.stats.release();
        reply.ok()
    }

    fn opendir(&mut self, _req: &fuse::Request, ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
        self.stats.opendir();
        let ino = Ino::from(ino);
        match ino.try_into() {
            Err(_) => reply.opened(0, 0),
            Ok(idx) => {
                let entry = self.get_entry(idx).unwrap();
                match &entry.get_type() {
                    EntryKind::Directory => reply.opened(0, 0),
                    _ => reply.error(libc::ENOTDIR),
                }
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        self.stats.readdir();
        let ino = Ino::from(ino);
        let finder = self.get_finder(ino).unwrap();
        let nb_entry = (finder.count().into_u64() + 2) as i64; // we include "." and ".."
        let mut readentry = ReadEntry::new(Rc::clone(self.arx.get_value_storage()), &finder);
        // If offset != 0, offset corresponds to what has already been seen. So we must start after.
        let offset = if offset == 0 { 0 } else { offset + 1 };
        if offset > 2 {
            // We skip offset entries (minus "." and "..")
            ReadEntry::skip(&mut readentry, jbk::EntryCount::from((offset - 2) as u32));
        }
        for i in offset..nb_entry {
            if i == 0 {
                reply.add(ino.get(), i as i64, fuse::FileType::Directory, ".");
            } else if i == 1 {
                let parent_ino = match ino.try_into() {
                    Err(_) => ino,
                    Ok(idx) => {
                        let entry = self.get_entry(idx).unwrap();
                        match entry.get_parent() {
                            None => Ino::from(1),
                            Some(parent_id) => parent_id.into(),
                        }
                    }
                };
                reply.add(parent_ino.get(), i as i64, fuse::FileType::Directory, "..");
            } else {
                match readentry.next() {
                    None => break,
                    Some(entry) => {
                        let entry = entry.unwrap();
                        let kind = match &entry.get_type() {
                            EntryKind::File => fuse::FileType::RegularFile,
                            EntryKind::Directory => fuse::FileType::Directory,
                            EntryKind::Link => fuse::FileType::Symlink,
                        };
                        // We remove "." and ".."
                        let entry_idx = finder.offset().into_u64() + (i as u64 - 2);
                        if reply.add(
                            entry_idx,
                            /* offset =*/ i,
                            kind,
                            entry.get_path().unwrap(),
                        ) {
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
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        reply: fuse::ReplyEmpty,
    ) {
        self.stats.releasedir();
        reply.ok()
    }
}

pub fn mount<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let mut stats = StatCounter::new();
    let arx = Arx::new(infile)?;
    let arxfs = ArxFs::new(arx, &mut stats)?;

    let options = ["-o", "-ro", "-o", "fsname=arx"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(arxfs, &outdir, &options)?;

    println!("Stats:\n {}", stats);
    Ok(())
}
