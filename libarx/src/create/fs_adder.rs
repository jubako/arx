use jubako as jbk;

use crate::create::{Creator, EntryKind, EntryTrait, Void};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::rc::Rc;

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
    fn new(path: PathBuf, name: OsString, recurse: bool, filter: Filter) -> jbk::Result<Box<Self>> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(Box::new(if attr.is_dir() {
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
        }))
    }

    pub fn new_from_fs(
        dir_entry: fs::DirEntry,
        recurse: bool,
        filter: Filter,
    ) -> jbk::Result<Box<Self>> {
        let path = dir_entry.path();
        let name = dir_entry.file_name();
        Ok(Box::new(if let Ok(file_type) = dir_entry.file_type() {
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
        }))
    }
}

impl EntryTrait for FsEntry {
    fn kind(self: Box<Self>) -> jbk::Result<EntryKind> {
        Ok(match self.kind {
            FsEntryKind::Dir => {
                let filter = Rc::clone(&self.filter);
                let recurse = self.recurse;
                EntryKind::Dir(Box::new(fs::read_dir(self.path.clone())?.map(
                    move |dir_entry| {
                        Ok(
                            FsEntry::new_from_fs(dir_entry?, recurse, Rc::clone(&filter))?
                                as Box<dyn EntryTrait + 'static>,
                        )
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

struct FakeDir {
    name: OsString,
    child: Option<Box<dyn EntryTrait + 'static>>,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

impl FakeDir {
    fn new(path: PathBuf, child: Option<Box<dyn EntryTrait>>) -> Self {
        let attr = fs::symlink_metadata(&path).unwrap();
        let name = path.file_name().unwrap().into();
        Self {
            name,
            uid: attr.uid() as u64,
            gid: attr.gid() as u64,
            mode: attr.mode() as u64,
            mtime: attr.mtime() as u64,
            child,
        }
    }
}

impl EntryTrait for FakeDir {
    fn kind(self: Box<Self>) -> jbk::Result<EntryKind> {
        Ok(EntryKind::Dir(Box::new(self.child.into_iter().map(Ok))))
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

fn build_entry_tree(base_path: PathBuf, mut components: std::path::Components) -> Box<FakeDir> {
    if let Some(component) = components.next() {
        // We have child, build it and then create ourself
        let mut child_path = base_path.clone();
        child_path.push(component);
        let entry = build_entry_tree(child_path, components);
        Box::new(FakeDir::new(base_path, Some(entry)))
    } else {
        // No child, just build ourself
        Box::new(FakeDir::new(base_path, None))
    }
}

pub struct FsAdder<'a> {
    creator: &'a mut Creator,
    strip_prefix: PathBuf,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut Creator, strip_prefix: PathBuf) -> Self {
        Self {
            creator,
            strip_prefix,
        }
    }

    pub fn add_from_path<P: AsRef<std::path::Path>>(&mut self, path: P, recurse: bool) -> Void {
        self.add_from_path_with_filter(path, recurse, Rc::new(&Some))
    }

    pub fn add_from_path_with_filter<P>(&mut self, path: P, recurse: bool, filter: Filter) -> Void
    where
        P: AsRef<std::path::Path>,
    {
        let rel_path = path.as_ref().strip_prefix(&self.strip_prefix).unwrap();
        if rel_path.as_os_str().is_empty() {
            if recurse {
                for sub_entry in fs::read_dir(path)? {
                    let sub_entry = sub_entry?;
                    self.creator.add_entry(FsEntry::new_from_fs(
                        sub_entry,
                        recurse,
                        Rc::clone(&filter),
                    )?)?;
                }
            }
            Ok(())
        } else if let Some(parents) = rel_path.parent() {
            let mut parent_tree = build_entry_tree(self.strip_prefix.clone(), parents.components());
            parent_tree.child = Some(FsEntry::new(
                path.as_ref().to_path_buf(),
                path.as_ref().file_name().unwrap().to_os_string(),
                recurse,
                filter,
            )?);
            self.creator.add_entry(parent_tree)
        } else {
            self.creator.add_entry(FsEntry::new(
                path.as_ref().to_path_buf(),
                path.as_ref().file_name().unwrap().to_os_string(),
                recurse,
                filter,
            )?)
        }
    }
}
