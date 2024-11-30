use std::collections::HashSet;
use std::fs::{create_dir, create_dir_all, OpenOptions};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::{AllProperties, Arx, Builder, Walker};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;
use jbk::reader::MayMissPack;

struct FileEntry {
    path: String,
    content: jbk::ContentAddress,
    mtime: u64,
}

struct Link {
    path: String,
    target: String,
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
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        let content = self.content_address_property.create(reader)?;
        let mtime = self.mtime_property.create(reader)?;
        Ok(FileEntry {
            path: String::from_utf8(path)?,
            content,
            mtime,
        })
    }
}

struct LinkBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
    link_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for LinkBuilder {
    type Entry = Link;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            link_property: properties.link_target_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;

        let target_prop = self.link_property.create(reader)?;
        let mut target = vec![];
        target_prop.resolve_to_vec(&mut target)?;
        Ok(Link {
            path: String::from_utf8(path)?,
            target: String::from_utf8(target)?,
        })
    }
}

struct DirBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for DirBuilder {
    type Entry = String;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(String::from_utf8(path)?)
    }
}

type FullBuilder = (FileBuilder, LinkBuilder, DirBuilder);

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "cmd_utils", derive(clap::ValueEnum))]
pub enum Overwrite {
    Skip,
    Warn,
    Newer,
    Overwrite,
    Error,
}

struct Extractor<'a, 'scope>
where
    'a: 'scope,
{
    arx: &'a Arx,
    scope: &'scope rayon::Scope<'a>,
    files: HashSet<crate::PathBuf>,
    base_dir: PathBuf,
    print_progress: bool,
    recurse: bool,
    extract_ok: Arc<AtomicBool>,
    overwrite: Overwrite,
}

impl Extractor<'_, '_> {
    fn should_extract(&self, current_file: &crate::Path, is_dir: bool) -> bool {
        if self.files.is_empty() {
            return true;
        }

        if self.files.contains(current_file) {
            return true;
        }

        if self.recurse {
            // We must extract any file/dir child of a directory to extract.
            let mut parent = current_file.parent();
            while let Some(p) = parent {
                if self.files.contains(p) {
                    return true;
                }
                parent = p.parent();
            }
        }

        if is_dir {
            // We must create any dirs parent of files/dirs to extract.
            for file in &self.files {
                let mut parent = file.parent();
                while let Some(p) = parent {
                    if current_file == p {
                        return true;
                    }
                    parent = p.parent();
                }
            }
        }
        false
    }
    fn abs_path(&self, current_file: &crate::Path) -> PathBuf {
        current_file.to_path(&self.base_dir)
    }
}

impl<'a, 'scope> crate::walk::Operator<crate::PathBuf, FullBuilder> for Extractor<'a, 'scope>
where
    'a: 'scope,
{
    fn on_start(&self, _current_path: &mut crate::PathBuf) -> jbk::Result<()> {
        create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut crate::PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_directory_enter(
        &self,
        current_path: &mut crate::PathBuf,
        path: &String,
    ) -> jbk::Result<bool> {
        current_path.push(path);
        if !self.should_extract(current_path, true) {
            return Ok(false);
        }
        let abs_path = self.abs_path(current_path);
        if !abs_path.try_exists()? {
            create_dir(&abs_path)?;
            if self.print_progress {
                println!("{}", abs_path.display());
            }
        }
        Ok(true)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut crate::PathBuf,
        _path: &String,
    ) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }

    fn on_file(&self, current_path: &mut crate::PathBuf, entry: &FileEntry) -> jbk::Result<()> {
        let mut current_path = current_path.clone();
        current_path.push(&entry.path);
        let entry_content = entry.content;
        let abs_path = self.abs_path(&current_path);
        let print_progress = self.print_progress;
        let arx = self.arx;
        if !self.should_extract(&current_path, false) {
            return Ok(());
        }
        let bytes = arx.container.get_bytes(entry_content).unwrap();
        let extract_ok = Arc::clone(&self.extract_ok);

        match bytes {
            MayMissPack::FOUND(bytes) => {
                let abs_path = abs_path.clone();
                let mut file = match OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&abs_path)
                {
                    Ok(f) => f,
                    Err(e) => match e.kind() {
                        ErrorKind::AlreadyExists => match self.overwrite {
                            Overwrite::Skip => return Ok(()),
                            Overwrite::Warn => {
                                eprintln!("File {} already exists.", abs_path.display());
                                return Ok(());
                            }
                            Overwrite::Newer => {
                                let existing_metadata = std::fs::metadata(&abs_path)?;
                                let existing_time = existing_metadata.modified()?;
                                let new_time =
                                    SystemTime::UNIX_EPOCH + Duration::from_secs(entry.mtime);
                                if new_time >= existing_time {
                                    OpenOptions::new()
                                        .write(true)
                                        .truncate(true)
                                        .open(&abs_path)?
                                } else {
                                    return Ok(());
                                }
                            }
                            Overwrite::Overwrite => OpenOptions::new()
                                .write(true)
                                .truncate(true)
                                .open(&abs_path)?,
                            Overwrite::Error => {
                                return Err(
                                    format!("File {} already exists.", abs_path.display()).into()
                                );
                            }
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
                        log::error!("Error writing content to {} : {}", abs_path.display(), e);
                        extract_ok.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                });
            }
            MayMissPack::MISSING(pack_info) => {
                log::error!(
                    "Missing pack {} for {}. Declared location is {}",
                    pack_info.uuid,
                    abs_path.display(),
                    String::from_utf8_lossy(&pack_info.pack_location)
                );
            }
        }

        if print_progress {
            println!("{}", abs_path.display());
        }

        Ok(())
    }
    fn on_link(&self, current_path: &mut crate::PathBuf, link: &Link) -> jbk::Result<()> {
        current_path.push(&link.path);
        if !self.should_extract(current_path, false) {
            current_path.pop();
            return Ok(());
        }
        let abs_path = self.abs_path(current_path);
        symlink(PathBuf::from(&link.target), PathBuf::from(&abs_path))?;
        if self.print_progress {
            println!("{}", abs_path.display());
        }
        current_path.pop();
        Ok(())
    }
}

pub fn extract(
    infile: &Path,
    outdir: &Path,
    files_to_extract: HashSet<crate::PathBuf>,
    recurse: bool,
    progress: bool,
    overwrite: Overwrite,
) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    extract_arx(&arx, outdir, files_to_extract, recurse, progress, overwrite)
}

pub fn extract_arx(
    arx: &Arx,
    outdir: &Path,
    files_to_extract: HashSet<crate::PathBuf>,
    recurse: bool,
    progress: bool,
    overwrite: Overwrite,
) -> jbk::Result<()> {
    let mut walker = Walker::new(arx, Default::default());
    let extract_ok = Arc::new(AtomicBool::new(true));
    rayon::scope(|scope| {
        let extractor = Extractor {
            arx,
            scope,
            files: files_to_extract,
            base_dir: outdir.to_path_buf(),
            print_progress: progress,
            extract_ok: Arc::clone(&extract_ok),
            recurse,
            overwrite,
        };
        walker.run(&extractor)
    })?;
    if !extract_ok.load(std::sync::atomic::Ordering::Relaxed) {
        Err("Some errors appends during extraction.".to_owned().into())
    } else {
        Ok(())
    }
}

pub fn extract_arx_range<R: jbk::reader::Range + Sync>(
    arx: &Arx,
    outdir: &Path,
    range: &R,
    files_to_extract: HashSet<crate::PathBuf>,
    recurse: bool,
    progress: bool,
    overwrite: Overwrite,
) -> jbk::Result<()> {
    let mut walker = Walker::new(arx, Default::default());
    let extract_ok = Arc::new(AtomicBool::new(true));
    rayon::scope(|scope| {
        let extractor = Extractor {
            arx,
            scope,
            files: files_to_extract,
            base_dir: outdir.to_path_buf(),
            print_progress: progress,
            extract_ok: Arc::clone(&extract_ok),
            recurse,
            overwrite,
        };
        walker.run_from_range(&extractor, range)
    })?;
    if !extract_ok.load(std::sync::atomic::Ordering::Relaxed) {
        Err("Some errors appends during extraction.".to_owned().into())
    } else {
        Ok(())
    }
}
