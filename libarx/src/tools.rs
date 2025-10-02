use core::convert::TryInto;
use core::ops::{Deref, DerefMut};
use std::collections::HashSet;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::ExtractError;
use crate::{AllProperties, Arx, ArxFormatError, Builder, Entry, Walker};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;
use jbk::reader::MayMissPack;
use std::sync::{Arc, Condvar, LazyLock, Mutex, OnceLock};

static FD_LIMIT: LazyLock<Arc<(Mutex<usize>, Condvar)>> =
    std::sync::LazyLock::new(|| Arc::new((Mutex::new(1000), Condvar::new())));

struct LimitedFile(std::fs::File);

impl Deref for LimitedFile {
    type Target = std::fs::File;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LimitedFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for LimitedFile {
    fn drop(&mut self) {
        let (lock, cvar) = &**FD_LIMIT;
        let mut fd_left = lock.lock().unwrap();
        *fd_left += 1;
        cvar.notify_one();
    }
}

trait OpenLimited {
    fn open_limited<P: AsRef<Path>>(&self, path: P) -> std::io::Result<LimitedFile>;
}

impl OpenLimited for std::fs::OpenOptions {
    fn open_limited<P: AsRef<Path>>(&self, path: P) -> std::io::Result<LimitedFile> {
        {
            let (lock, cvar) = &**FD_LIMIT;
            let mut fd_left = cvar
                .wait_while(lock.lock().unwrap(), |fd_left| *fd_left == 0)
                .unwrap();
            *fd_left -= 1;
        }
        Ok(LimitedFile(self.open(path)?))
    }
}

struct FileEntry {
    path: jbk::SmallString,
    content: jbk::ContentAddress,
    mtime: u64,
}

struct Link {
    path: jbk::SmallString,
    target: jbk::SmallString,
    mtime: u64,
}

struct FileBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
    mtime_property: jbk::reader::builder::IntProperty,
}

impl Builder for FileBuilder {
    type Entry = FileEntry;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            content_address_property: properties.file_content_address_property,
            mtime_property: properties.mtime_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = jbk::SmallBytes::new();
        path_prop.resolve_to_vec(&mut path)?;
        let content = self.content_address_property.create(reader)?;
        let mtime = self.mtime_property.create(reader)?;
        Ok(FileEntry {
            path: path.try_into()?,
            content,
            mtime,
        })
    }
}

struct LinkBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
    link_property: jbk::reader::builder::ArrayProperty,
    mtime_property: jbk::reader::builder::IntProperty,
}

impl Builder for LinkBuilder {
    type Entry = Link;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            link_property: properties.link_target_property.clone(),
            mtime_property: properties.mtime_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = jbk::SmallBytes::new();
        path_prop.resolve_to_vec(&mut path)?;

        let target_prop = self.link_property.create(reader)?;
        let mut target = jbk::SmallBytes::new();
        target_prop.resolve_to_vec(&mut target)?;
        let mtime = self.mtime_property.create(reader)?;
        Ok(Link {
            path: path.try_into()?,
            target: target.try_into()?,
            mtime,
        })
    }
}

struct DirBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for DirBuilder {
    type Entry = jbk::SmallString;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = jbk::SmallBytes::new();
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path.try_into()?)
    }
}

type FullBuilder = (FileBuilder, LinkBuilder, DirBuilder);

pub trait FileFilter: Send {
    ///  Should we accept (to extract) path
    fn accept(&self, path: &crate::Path) -> bool;

    /// Weither we early exit (don't enter the directory) if the directory is not accepted.
    /// true if we don't want to extract any file/directory under a refused directory
    /// false if we may still extract a file under a refused directory
    fn early_exit(&self) -> bool {
        true
    }
}

impl FileFilter for HashSet<crate::PathBuf> {
    fn accept(&self, path: &crate::Path) -> bool {
        self.contains(path)
    }
}

impl FileFilter for () {
    fn accept(&self, _path: &crate::Path) -> bool {
        true
    }
}

impl FileFilter for Box<dyn FileFilter> {
    fn accept(&self, path: &crate::Path) -> bool {
        self.as_ref().accept(path)
    }

    fn early_exit(&self) -> bool {
        self.as_ref().early_exit()
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "cmd_utils", derive(clap::ValueEnum))]
pub enum Overwrite {
    Skip,
    Warn,
    Newer,
    Overwrite,
    Error,
}

pub struct Extractor<'a, 'scope, F>
where
    'a: 'scope,
    F: FileFilter,
{
    arx: &'a Arx,
    root: jbk::EntryRange,
    scope: &'scope rayon::Scope<'a>,
    err: Arc<OnceLock<jbk::Error>>,
    filter: F,
    base_dir: PathBuf,
    print_progress: bool,
    overwrite: Overwrite,
}

impl<'a, 'scope, F> Extractor<'a, 'scope, F>
where
    'a: 'scope,
    F: FileFilter,
{
    fn create_parents(&self, current_file: &crate::Path) -> jbk::Result<()> {
        if let Some(parent_path) = current_file.parent() {
            let absolute_path = self.abs_path(parent_path);
            create_dir_all(absolute_path)?;
        }
        Ok(())
    }

    fn abs_path(&self, current_file: &crate::Path) -> PathBuf {
        current_file.to_path(&self.base_dir)
    }

    pub fn extract(&self, path: &crate::Path, recursive: bool) -> Result<(), ExtractError> {
        let entry = self
            .arx
            .get_entry_in_range::<FullBuilder, _>(path, &self.root)?;
        match &entry {
            Entry::File(e) => self.write_file(e, path),
            Entry::Link(e) => self.write_link(e, path),
            Entry::Dir(range, _e) => {
                self.write_dir(path)?;
                if recursive {
                    let mut walker = Walker::new(self.arx, path.to_relative_path_buf());
                    walker.run_from_range(self, range)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn extract_all(&self) -> Result<(), ExtractError> {
        let mut walker = Walker::new(self.arx, Default::default());
        walker.run_from_range(self, &self.root)
    }

    fn write_file(&self, entry: &FileEntry, path: &crate::Path) -> Result<(), ExtractError> {
        self.create_parents(path)?;
        let path = self.abs_path(path);

        let entry_content = entry.content;
        let print_progress = self.print_progress;
        let arx = self.arx;
        let bytes = arx
            .container
            .get_bytes(entry_content)?
            .and_then(|m| m.transpose())
            .ok_or(ArxFormatError(
                "Entry Content should point to valid content",
            ))?;
        let error = Arc::clone(&self.err);

        match bytes {
            MayMissPack::FOUND(bytes) => {
                let mut file = match OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open_limited(&path)
                {
                    Ok(f) => f,
                    Err(e) => match e.kind() {
                        ErrorKind::AlreadyExists => match self.overwrite {
                            Overwrite::Skip => return Ok(()),
                            Overwrite::Warn => {
                                eprintln!("File {} already exists.", path.display());
                                return Ok(());
                            }
                            Overwrite::Newer => {
                                let existing_metadata = std::fs::metadata(&path)?;
                                let existing_time = existing_metadata.modified()?;
                                let new_time =
                                    SystemTime::UNIX_EPOCH + Duration::from_secs(entry.mtime);
                                if new_time >= existing_time {
                                    OpenOptions::new()
                                        .write(true)
                                        .truncate(true)
                                        .open_limited(&path)?
                                } else {
                                    return Ok(());
                                }
                            }
                            Overwrite::Overwrite => OpenOptions::new()
                                .write(true)
                                .truncate(true)
                                .open_limited(&path)?,
                            Overwrite::Error => return Err(ExtractError::FileExists { path }),
                        },
                        _ => return Err(e.into()),
                    },
                };

                self.scope.spawn(move |_scope| {
                    // Don't use std::io::copy as it use an internal buffer where it read data into before writing in file.
                    // If content is compressed, we already have a buffer. Same thing for uncompress as the cluster is probably mmapped.
                    let size = bytes.size().into_u64();
                    let mut offset = 0;
                    let mut write_function = move || -> jbk::Result<()> {
                        loop {
                            let sub_size = std::cmp::min(size - offset, 4 * 1024) as usize;
                            let written = file.write(&bytes.get_slice(offset.into(), sub_size)?)?;
                            offset += written as u64;
                            if offset == size {
                                break;
                            }
                        }
                        Ok(())
                    };
                    if let Err(e) = write_function() {
                        let _ = error.set(e);
                    }
                });
            }
            MayMissPack::MISSING(pack_info) => {
                log::error!(
                    "Missing pack {} for {}. Declared location is {}",
                    pack_info.uuid,
                    path.display(),
                    pack_info.pack_location
                );
            }
        }

        if print_progress {
            println!("{}", path.display());
        }

        Ok(())
    }

    fn write_link(&self, link: &Link, path: &crate::Path) -> Result<(), ExtractError> {
        self.create_parents(path)?;
        let abs_path = self.abs_path(path);
        if let Err(e) = symlink(
            PathBuf::from(link.target.as_str()),
            PathBuf::from(&abs_path),
        ) {
            match e.kind() {
                ErrorKind::AlreadyExists => match self.overwrite {
                    Overwrite::Skip => return Ok(()),
                    Overwrite::Warn => {
                        eprintln!("Link {} already exists.", abs_path.display());
                        return Ok(());
                    }
                    Overwrite::Newer => {
                        let existing_metadata = std::fs::symlink_metadata(&abs_path)?;
                        let existing_time = existing_metadata.modified()?;
                        let new_time = SystemTime::UNIX_EPOCH + Duration::from_secs(link.mtime);
                        if new_time >= existing_time {
                            std::fs::remove_file(&abs_path)?;
                            symlink(
                                PathBuf::from(link.target.as_str()),
                                PathBuf::from(&abs_path),
                            )?;
                        } else {
                            return Ok(());
                        }
                    }
                    Overwrite::Overwrite => {
                        std::fs::remove_file(&abs_path)?;
                        symlink(
                            PathBuf::from(link.target.as_str()),
                            PathBuf::from(&abs_path),
                        )?;
                    }
                    Overwrite::Error => return Err(e.into()),
                },
                _ => return Err(e.into()),
            }
        }
        if self.print_progress {
            println!("{}", abs_path.display());
        }
        Ok(())
    }

    fn write_dir(&self, path: &crate::Path) -> Result<(), ExtractError> {
        self.create_parents(path)?;
        let abs_path = self.abs_path(path);
        create_dir_all(&abs_path)?;
        if self.print_progress {
            println!("{}", abs_path.display());
        }
        Ok(())
    }

    pub fn finish(self) -> Arc<OnceLock<jbk::Error>> {
        self.err
    }
}

impl<'a, 'scope, F> crate::walk::Operator<crate::PathBuf, FullBuilder> for Extractor<'a, 'scope, F>
where
    'a: 'scope,
    F: FileFilter,
{
    type Error = ExtractError;
    fn on_start(&self, _current_path: &mut crate::PathBuf) -> Result<(), ExtractError> {
        create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut crate::PathBuf) -> Result<(), ExtractError> {
        Ok(())
    }

    fn on_directory_enter(
        &self,
        current_path: &mut crate::PathBuf,
        path: &jbk::SmallString,
    ) -> Result<bool, ExtractError> {
        current_path.push(path.as_str());
        if !self.filter.accept(current_path) {
            return Ok(!self.filter.early_exit());
        }
        self.write_dir(current_path)?;
        Ok(true)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut crate::PathBuf,
        _path: &jbk::SmallString,
    ) -> Result<(), ExtractError> {
        current_path.pop();
        Ok(())
    }

    fn on_file(
        &self,
        current_path: &mut crate::PathBuf,
        entry: &FileEntry,
    ) -> Result<(), ExtractError> {
        let mut current_path = current_path.clone();
        current_path.push(entry.path.as_str());
        if !self.filter.accept(&current_path) {
            return Ok(());
        }
        self.write_file(entry, &current_path)
    }

    fn on_link(&self, current_path: &mut crate::PathBuf, link: &Link) -> Result<(), ExtractError> {
        let mut current_path = current_path.clone();
        current_path.push(link.path.as_str());
        if !self.filter.accept(&current_path) {
            return Ok(());
        }
        self.write_link(link, &current_path)
    }
}

pub fn extract_all(
    infile: &Path,
    outdir: &Path,
    progress: bool,
    overwrite: Overwrite,
) -> Result<(), ExtractError> {
    let arx = Arx::new(infile)?;
    ExtractBuilder::new(outdir)
        .overwrite(overwrite)
        .progress(progress)
        .extract(&arx, None)
}

pub struct ExtractBuilder<'a, F, P> {
    outdir: &'a Path,
    items: P,
    filter: F,
    recursive: bool,
    progress: bool,
    overwrite: Overwrite,
}

impl<'a> ExtractBuilder<'a, (), ()> {
    pub fn new(outdir: &'a Path) -> Self {
        Self {
            outdir,
            items: (),
            filter: (),
            recursive: true,
            progress: false,
            overwrite: Overwrite::Warn,
        }
    }
}

impl<'a, P> ExtractBuilder<'a, (), P> {
    pub fn filter<F: FileFilter>(self, filter: F) -> ExtractBuilder<'a, F, P> {
        ExtractBuilder {
            outdir: self.outdir,
            items: self.items,
            filter,
            recursive: self.recursive,
            progress: self.progress,
            overwrite: self.overwrite,
        }
    }
}

impl<'a, F> ExtractBuilder<'a, F, ()> {
    pub fn items<P: AsRef<crate::Path> + Sync>(
        self,
        items: &'a [P],
        recursive: bool,
    ) -> ExtractBuilder<'a, F, &'a [P]> {
        ExtractBuilder {
            outdir: self.outdir,
            items,
            filter: self.filter,
            recursive,
            progress: self.progress,
            overwrite: self.overwrite,
        }
    }
}

impl<'a, F, P> ExtractBuilder<'a, F, P> {
    pub fn overwrite(self, overwrite: Overwrite) -> ExtractBuilder<'a, F, P> {
        ExtractBuilder {
            outdir: self.outdir,
            items: self.items,
            filter: self.filter,
            recursive: self.recursive,
            progress: self.progress,
            overwrite,
        }
    }
}

impl<'a, F, P> ExtractBuilder<'a, F, P> {
    pub fn progress(self, progress: bool) -> ExtractBuilder<'a, F, P> {
        ExtractBuilder {
            outdir: self.outdir,
            items: self.items,
            filter: self.filter,
            recursive: self.recursive,
            progress,
            overwrite: self.overwrite,
        }
    }
}

impl<'a, F> ExtractBuilder<'a, F, ()>
where
    F: FileFilter,
{
    pub fn extract(self, arx: &Arx, root: Option<&crate::Path>) -> Result<(), ExtractError> {
        self.items(&[] as &[&crate::Path], true).extract(arx, root)
    }
}

impl<'a, F, P> ExtractBuilder<'a, F, &[P]>
where
    F: FileFilter,
    P: AsRef<crate::Path> + Sync,
{
    pub fn extract(self, arx: &Arx, root: Option<&crate::Path>) -> Result<(), ExtractError> {
        let root = match root {
            None => jbk::EntryRange::from_range(&arx.root_index),
            Some(p) => {
                let root = arx.get_entry::<((), (), ())>(p)?;
                match root {
                    crate::Entry::Dir(range, _) => jbk::EntryRange::from_range(&range),
                    _ => {
                        return Err(ExtractError::RootNotDir {
                            path: p.to_relative_path_buf(),
                        })
                    }
                }
            }
        };
        self.extract_root(arx, root)
    }

    fn extract_root(self, arx: &Arx, root: jbk::EntryRange) -> Result<(), ExtractError> {
        let error = rayon::scope(|scope| -> Result<Arc<OnceLock<jbk::Error>>, ExtractError> {
            let extractor = Extractor {
                arx,
                root,
                scope,
                err: Default::default(),
                filter: self.filter,
                base_dir: self.outdir.to_path_buf(),
                print_progress: self.progress,
                overwrite: self.overwrite,
            };
            if self.items.is_empty() {
                extractor.extract_all()?
            } else {
                for item in self.items.iter() {
                    extractor.extract(item.as_ref(), self.recursive)?;
                }
            }
            Ok(extractor.finish())
        })?;
        match Arc::into_inner(error)
            .expect("No one should have a ref to err.")
            .take()
        {
            None => Ok(()),
            Some(e) => Err(e.into()),
        }
    }
}
