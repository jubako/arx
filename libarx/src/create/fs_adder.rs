use crate::create::{EntryKind, EntryTrait, SimpleCreator, Void};
use jbk::creator::InputReader;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use std::{fs, io::Cursor};

pub enum FsEntryKind {
    Dir,
    File(jbk::Size, jbk::ContentAddress),
    Link,
    Other,
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
    pub fn new_from_walk_entry<A: jbk::creator::ContentAdder>(
        dir_entry: walkdir::DirEntry,
        arx_path: crate::PathBuf,
        adder: &mut A,
    ) -> jbk::Result<Box<Self>> {
        let fs_path = dir_entry.path().to_path_buf();
        let attr = dir_entry.metadata().unwrap();
        let kind = if attr.is_dir() {
            FsEntryKind::Dir
        } else if attr.is_file() {
            let reader: Box<dyn InputReader> = if attr.len() < 1024 * 1024 {
                let content = std::fs::read(&fs_path)?;
                Box::new(Cursor::new(content))
            } else {
                Box::new(jbk::creator::InputFile::open(&fs_path)?)
            };
            let content_address = adder.add_content(reader, jbk::creator::CompHint::Detect)?;
            FsEntryKind::File(attr.len().into(), content_address)
        } else if attr.is_symlink() {
            FsEntryKind::Link
        } else {
            FsEntryKind::Other
        };
        Ok(Box::new(Self {
            kind,
            fs_path,
            arx_path,
            #[cfg(unix)]
            uid: attr.uid() as u64,
            #[cfg(windows)]
            uid: 1000,
            #[cfg(unix)]
            gid: attr.gid() as u64,
            #[cfg(windows)]
            gid: 1000,
            #[cfg(unix)]
            mode: attr.mode() as u64,
            #[cfg(windows)]
            mode: 755,
            #[cfg(unix)]
            mtime: attr.mtime() as u64,
            #[cfg(windows)]
            mtime: epochs::to_unix(epochs::windows_file(attr.last_write_time() as i64).unwrap())
                as u64,
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
