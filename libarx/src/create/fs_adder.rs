use crate::create::{EntryKind, EntryTrait, SimpleCreator, Void};
use jbk::creator::InputReader;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub enum FsEntryKind {
    Dir,
    File(jbk::Size, jbk::ContentAddress),
    Link,
    Other,
}

pub trait Adder {
    fn add<R: jbk::creator::InputReader>(&mut self, reader: R) -> jbk::Result<jbk::ContentAddress>;
}

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub fs_path: PathBuf,
    pub arx_path: crate::PathBuf,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl FsEntry {
    pub fn new_from_walk_entry<A: Adder>(
        dir_entry: walkdir::DirEntry,
        arx_path: crate::PathBuf,
        adder: &mut A,
    ) -> jbk::Result<Box<Self>> {
        let fs_path = dir_entry.path().to_path_buf();
        let attr = dir_entry.metadata().unwrap();
        let kind = if attr.is_dir() {
            FsEntryKind::Dir
        } else if attr.is_file() {
            let reader = jbk::creator::InputFile::open(&fs_path)?;
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

            FsEntryKind::Link => {
                let target = fs::read_link(&self.fs_path)?;
                Some(EntryKind::Link(
                    crate::PathBuf::from_path(&target)
                        .unwrap_or_else(|_| panic!("{target:?} must be a relative utf-8 path")),
                ))
            }
            _ => None,
        })
    }
    fn path(&self) -> &crate::Path {
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
    creator: &'a mut SimpleCreator,
    strip_prefix: crate::PathBuf,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut SimpleCreator, strip_prefix: crate::PathBuf) -> Self {
        Self {
            creator,
            strip_prefix,
        }
    }

    pub fn add_from_path<P>(&mut self, path: P, recurse: bool) -> Void
    where
        P: AsRef<std::path::Path>,
    {
        self.add_from_path_with_filter(path, recurse, |_e| true)
    }

    pub fn add_from_path_with_filter<P, F>(&mut self, path: P, recurse: bool, filter: F) -> Void
    where
        P: AsRef<std::path::Path>,
        F: FnMut(&walkdir::DirEntry) -> bool,
    {
        let mut walker = walkdir::WalkDir::new(path);
        if !recurse {
            walker = walker.max_depth(0);
        }
        let walker = walker.into_iter();
        for entry in walker.filter_entry(filter) {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let arx_path = crate::PathBuf::from_path(entry_path)
                .unwrap_or_else(|_| panic!("{entry_path:?} must be a relative utf-8 path."));
            let arx_path: crate::PathBuf =
                arx_path.strip_prefix(&self.strip_prefix).unwrap().into();
            if arx_path.as_str().is_empty() {
                continue;
            }
            let entry = FsEntry::new_from_walk_entry(entry, arx_path, self.creator.adder())?;
            self.creator.add_entry(entry.as_ref())?;
        }
        Ok(())
    }
}
