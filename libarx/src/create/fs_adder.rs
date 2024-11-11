use crate::create::{EntryKind, EntryTrait, SimpleCreator, Void};
use bstr::{BString, ByteVec};
use jbk::creator::InputReader;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use std::{io::Cursor, sync::mpsc, thread::spawn};

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
) -> jbk::Result<(DetectedEntryKind, Metadata)> {
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
    ) -> jbk::Result<Box<Self>> {
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
    fn kind(&self) -> jbk::Result<Option<EntryKind>> {
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
        F: Send + 'static,
    {
        let path = path.as_ref();
        log::trace!("add_from_path_with_filter(path:{path:?}, recurse:{recurse})");
        let (tx, rx) = mpsc::channel();
        let path_copy = path.to_path_buf();
        let strip_prefix = self.strip_prefix.clone();

        spawn(move || {
            let mut walker = walkdir::WalkDir::new(path_copy);

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
            // Walkdir behaves differently if root is a link to a directory
            let is_root_entry = entry.path() == path;

            let entry_path = entry.path();
            let arx_path = match crate::PathBuf::from_path(entry_path) {
                Ok(p) => p,
                Err(e) => {
                    return Err(match e.kind() {
                        relative_path::FromPathErrorKind::NonRelative => {
                            format!("{} is not a relative path", entry_path.display())
                        }
                        relative_path::FromPathErrorKind::NonUtf8 => {
                            format!("Non utf8 char in {}", entry_path.display())
                        }
                        relative_path::FromPathErrorKind::BadSeparator => {
                            format!("Invalid path separator in {}", entry_path.display(),)
                        }
                        _ => {
                            format!(
                                "Unknown error converting {} to relative utf-8 path.",
                                entry_path.display()
                            )
                        }
                    }
                    .into())
                }
            };
            let arx_path: crate::PathBuf = match arx_path.strip_prefix(&strip_prefix) {
                Ok(p) => p,
                Err(_e) => return Err(format!("{strip_prefix} is not in {arx_path}").into()),
            }
            .into();
            if arx_path.as_str().is_empty() {
                continue;
            }

            let entry = FsEntry::new_from_path(
                entry.path(),
                arx_path,
                self.creator.adder(),
                is_root_entry,
            )?;

            self.creator.add_entry(entry.as_ref())?;
        }
        Ok(())
    }
}
