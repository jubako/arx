use crate::create::{EntryKind, EntryTrait, SimpleCreator, Void};
use crate::{CreatorError, InputError};
use bstr::{BString, ByteVec};
use jbk::creator::InputReader;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use std::{borrow::Cow, io::Cursor, sync::mpsc, thread::spawn};

#[derive(Debug)]
pub enum FsEntryKind {
    Dir,
    File(jbk::Size, jbk::ContentAddress),
    Link(PathBuf),
    Other,
}

#[derive(Debug)]
pub enum DetectedEntryKind {
    Dir,
    File(u64, PathBuf),
    Link(PathBuf),
    Other,
}

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub arx_path: crate::PathBuf,
    uid: u64,
    gid: u64,
    mode: u64,
    mtime: u64,
}

fn detect_kind(
    mut path: PathBuf,
    follow_symlink: bool,
) -> Result<(DetectedEntryKind, Metadata), std::io::Error> {
    log::trace!("std::fs::symlink_metadata({path:?})");
    let mut attr = std::fs::symlink_metadata(&path)?;
    log::trace!("=> {attr:?}");

    if attr.is_symlink() && follow_symlink {
        path = std::fs::canonicalize(path)?;
        log::trace!("New path is {path:?}");
        attr = std::fs::symlink_metadata(&path)?;
        log::trace!("New attr is {attr:?}");
    }
    let kind = if attr.is_dir() {
        DetectedEntryKind::Dir
    } else if attr.is_file() {
        DetectedEntryKind::File(attr.len(), path)
    } else if attr.is_symlink() {
        DetectedEntryKind::Link(path)
    } else {
        DetectedEntryKind::Other
    };
    Ok((kind, attr))
}

impl FsEntry {
    pub fn new_from_path<A: jbk::creator::ContentAdder>(
        fs_path: &std::path::Path,
        arx_path: crate::PathBuf,
        adder: &mut A,
        follow_symlink: bool,
    ) -> Result<Box<Self>, CreatorError> {
        let (kind, attr) = detect_kind(fs_path.to_path_buf(), follow_symlink)?;
        let kind = match kind {
            DetectedEntryKind::Dir => FsEntryKind::Dir,
            DetectedEntryKind::File(file_size, path) => {
                let reader: Box<dyn InputReader> = if file_size < 1024 * 1024 {
                    let content = std::fs::read(&path)?;
                    Box::new(Cursor::new(content))
                } else {
                    Box::new(jbk::creator::InputFile::open(&path)?)
                };
                let content_address = adder.add_content(reader, jbk::creator::CompHint::Detect)?;
                FsEntryKind::File(file_size.into(), content_address)
            }
            DetectedEntryKind::Link(path) => FsEntryKind::Link(std::fs::read_link(&path)?),
            DetectedEntryKind::Other => FsEntryKind::Other,
        };
        log::debug!("{fs_path:?} is dectected as a {kind:?}");
        Ok(Box::new(Self {
            kind,
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
    fn kind(&self) -> Result<Option<EntryKind>, CreatorError> {
        Ok(match &self.kind {
            FsEntryKind::Dir => Some(EntryKind::Dir),
            FsEntryKind::File(size, content_address) => {
                Some(EntryKind::File(*size, *content_address))
            }

            FsEntryKind::Link(target) => Some(EntryKind::Link(BString::from(
                Vec::from_path_buf(target.clone())
                    .unwrap_or_else(|target| panic!("{target:?} must be utf-8")),
            ))),
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

struct Trimer<'a>(Option<&'a std::path::Path>);

impl<'a> Trimer<'a> {
    fn new(keep_parent: bool, p: &'a std::path::Path) -> Self {
        if keep_parent {
            if p.is_absolute() {
                let mut components = p.components();
                let root_prefix = components
                    .next()
                    .expect("Absolute path should have at least a component.");
                Trimer(Some(std::path::Path::new(root_prefix.as_os_str())))
            } else {
                Trimer(None)
            }
        } else {
            let mut ancestors = p.ancestors();
            ancestors.next();
            let parent_to_strip = ancestors.next();
            Trimer(parent_to_strip)
        }
    }
    fn trim<'s, 'p>(&'s self, path: &'p std::path::Path) -> &'p std::path::Path {
        if let Some(prefix) = self.0 {
            path.strip_prefix(prefix)
                .expect("Prefix should be a prefix of entry.path()")
        } else {
            path
        }
    }
}

fn to_arx_path(path: &std::path::Path) -> Result<Cow<'_, crate::Path>, InputError> {
    let ret = match crate::Path::from_path(path) {
        Ok(p) => Ok(p.into()),
        Err(e) => {
            if let relative_path::FromPathErrorKind::BadSeparator = e.kind() {
                crate::PathBuf::from_path(path).map(|p| p.into())
            } else {
                Err(e)
            }
        }
    };
    ret.map_err(|e| match e.kind() {
        relative_path::FromPathErrorKind::NonRelative => {
            InputError(format!("{} is not a relative path", path.display()))
        }
        relative_path::FromPathErrorKind::NonUtf8 => {
            InputError(format!("Non utf8 char in {}", path.display()))
        }
        relative_path::FromPathErrorKind::BadSeparator => {
            InputError(format!("Invalid path separator in {}", path.display()))
        }
        _ => InputError(format!(
            "Unknown error converting {} to relative utf-8 path.",
            path.display()
        )),
    })
}

pub struct FsAdder<'a> {
    creator: &'a mut SimpleCreator,
    keep_parents: bool,
    follow_symlink: bool,
    dir_as_root: bool,
}

impl<'a> FsAdder<'a> {
    pub fn new(
        creator: &'a mut SimpleCreator,
        keep_parents: bool,
        follow_symlink: bool,
        dir_as_root: bool,
    ) -> Self {
        Self {
            creator,
            keep_parents,
            follow_symlink,
            dir_as_root,
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
        F: Send + 'static,
    {
        let path = path.as_ref();
        log::trace!("add_from_path_with_filter(path:{path:?}, recurse:{recurse})");
        let (tx, rx) = mpsc::channel();
        let path_copy = path.to_path_buf();
        let follow_symlink = self.follow_symlink;
        let trimmer = Trimer::new(self.keep_parents, path);

        spawn(move || {
            let mut walker = walkdir::WalkDir::new(path_copy).follow_links(follow_symlink);

            if !recurse {
                walker = walker.max_depth(0);
            }
            let walker = walker.into_iter();
            for entry in walker.filter_entry(filter) {
                let entry = entry.unwrap();
                tx.send(entry).unwrap();
            }
        });

        while let Ok(entry) = rx.recv() {
            // We always want to follow link if this is the root entry.
            // This allow user to create a link to a file/dir to add the entry under a different name.
            // Walkdir will do the same anyway if it is a directory.
            let is_root_entry = entry.path() == path;
            let entry_path = if self.dir_as_root {
                if is_root_entry {
                    continue;
                }
                entry
                    .path()
                    .strip_prefix(path)
                    .expect("Entry path is a chird of path")
            } else {
                trimmer.trim(entry.path())
            };
            let arx_path = to_arx_path(entry_path)?;
            self.add_entry_from_path(entry.path(), &arx_path, is_root_entry)?;
        }
        Ok(())
    }

    pub fn add_from_list<Iter>(&mut self, paths: Iter) -> Void
    where
        Iter: Iterator<Item = std::path::PathBuf>,
    {
        for path in paths {
            let trimer = Trimer::new(self.keep_parents, &path);
            let arx_path = trimer.trim(&path);
            let arx_path = to_arx_path(arx_path)?;
            self.add_entry_from_path(&path, &arx_path, false)?;
        }
        Ok(())
    }

    pub fn add_entry_from_path(
        &mut self,
        path: &std::path::Path,
        arx_path: &crate::Path,
        is_root_dir: bool,
    ) -> Void {
        log::debug!("add_path(path:{path:?}, arx_path: {arx_path:?}, is_root_dir:{is_root_dir})");
        if arx_path.as_str().is_empty() {
            return Ok(());
        }
        let entry = FsEntry::new_from_path(
            path,
            arx_path.into(),
            self.creator.adder(),
            self.follow_symlink || is_root_dir,
        )?;

        self.creator.add_entry(entry.as_ref())
    }
}
