use jubako as jbk;

use crate::create::{EntryKind, EntryStoreCreator, EntryTrait, Void};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub enum FsEntryKind {
    Dir,
    File(jbk::Size, jbk::ContentAddress),
    Link,
    Other,
}

pub trait Adder {
    fn add(&mut self, reader: jbk::Reader) -> jbk::Result<jbk::ContentAddress>;
}

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub fs_path: PathBuf,
    pub arx_path: PathBuf,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl FsEntry {
    pub fn new_from_walk_entry(
        dir_entry: walkdir::DirEntry,
        arx_path: PathBuf,
        adder: &mut dyn Adder,
    ) -> jbk::Result<Box<Self>> {
        let fs_path = dir_entry.path().to_path_buf();
        let attr = dir_entry.metadata().unwrap();
        let kind = if attr.is_dir() {
            FsEntryKind::Dir
        } else if attr.is_file() {
            let reader: jbk::Reader = jbk::creator::FileSource::open(&fs_path)?.into();
            let size = reader.size();
            let content_address = adder.add(reader)?;
            FsEntryKind::File(size, content_address)
        } else if attr.is_symlink() {
            FsEntryKind::Link
        } else {
            FsEntryKind::Other
        };
        Ok(Box::new(Self {
            kind,
            fs_path,
            arx_path,
            uid: attr.uid() as u64,
            gid: attr.gid() as u64,
            mode: attr.mode() as u64,
            mtime: attr.mtime() as u64,
        }))
    }
}

impl EntryTrait for FsEntry {
    fn kind(&self) -> jbk::Result<Option<EntryKind>> {
        Ok(match self.kind {
            FsEntryKind::Dir => Some(EntryKind::Dir),
            FsEntryKind::File(size, content_address) => {
                Some(EntryKind::File(size, content_address))
            }

            FsEntryKind::Link => Some(EntryKind::Link(fs::read_link(&self.fs_path)?.into())),
            _ => None,
        })
    }
    fn path(&self) -> &std::path::Path {
        &self.arx_path
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

pub struct FsAdder<'a> {
    creator: &'a mut EntryStoreCreator,
    strip_prefix: &'a Path,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut EntryStoreCreator, strip_prefix: &'a Path) -> Self {
        Self {
            creator,
            strip_prefix,
        }
    }

    pub fn add_from_path<P, A>(&mut self, path: P, recurse: bool, adder: &mut A) -> Void
    where
        P: AsRef<std::path::Path>,
        A: Adder,
    {
        self.add_from_path_with_filter(path, recurse, |_e| true, adder)
    }

    pub fn add_from_path_with_filter<P, F, A>(
        &mut self,
        path: P,
        recurse: bool,
        filter: F,
        adder: &mut A,
    ) -> Void
    where
        P: AsRef<std::path::Path>,
        F: FnMut(&walkdir::DirEntry) -> bool,
        A: Adder,
    {
        let mut walker = walkdir::WalkDir::new(path);
        if !recurse {
            walker = walker.max_depth(0);
        }
        let walker = walker.into_iter();
        for entry in walker.filter_entry(filter) {
            let entry = entry.unwrap();
            let arx_path = entry
                .path()
                .strip_prefix(self.strip_prefix)
                .unwrap()
                .to_path_buf();
            if arx_path.as_os_str().is_empty() {
                continue;
            }
            let entry = FsEntry::new_from_walk_entry(entry, arx_path, adder)?;
            self.creator.add_entry(entry.as_ref())?;
        }
        Ok(())
    }
}