use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{create_dir, create_dir_all, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use crate::{AllProperties, Arx, Builder, Reader, Walker};
use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;

type U8Path = Vec<u8>;

struct FileEntry {
    path: U8Path,
    content: jbk::ContentAddress,
}

struct Link {
    path: U8Path,
    target: U8Path,
}

struct FileBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl Builder for FileBuilder {
    type Entry = FileEntry;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            content_address_property: properties.file_content_address_property,
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        let content = self.content_address_property.create(reader)?;
        Ok(FileEntry { path, content })
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

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;

        let target_prop = self.link_property.create(reader)?;
        let mut target = vec![];
        target_prop.resolve_to_vec(&mut target)?;
        Ok(Link { path, target })
    }
}

struct DirBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for DirBuilder {
    type Entry = U8Path;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

type FullBuilder = (FileBuilder, LinkBuilder, DirBuilder);

struct Extractor<'a> {
    arx: &'a Arx,
    files: HashSet<PathBuf>,
    base_dir: PathBuf,
    print_progress: bool,
}

impl Extractor<'_> {
    fn should_extract(&self, current_file: &PathBuf, is_dir: bool) -> bool {
        if self.files.is_empty() {
            return true;
        }
        if self.files.contains(current_file) {
            return true;
        } else if is_dir {
            for file in &self.files {
                // We must create the dir if it is the parent dir of the file to extract
                for ancestor in file.ancestors() {
                    if current_file == ancestor {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn abs_path(&self, current_file: &PathBuf) -> PathBuf {
        [&self.base_dir, current_file].iter().collect()
    }
}

impl crate::walk::Operator<PathBuf, FullBuilder> for Extractor<'_> {
    fn on_start(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, path: &U8Path) -> jbk::Result<bool> {
        current_path.push(OsString::from_vec(path.clone()));
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
    fn on_directory_exit(&self, current_path: &mut PathBuf, _path: &U8Path) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut PathBuf, entry: &FileEntry) -> jbk::Result<()> {
        let reader = self.arx.container.get_reader(entry.content)?;
        current_path.push(OsString::from_vec(entry.path.clone()));
        if !self.should_extract(current_path, false) {
            current_path.pop();
            return Ok(());
        }
        let abs_path = self.abs_path(current_path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&abs_path)?;
        let size = reader.size().into_usize();
        let mut offset = 0;
        loop {
            let sub_size = std::cmp::min(size - offset, 4 * 1024);
            let reader = reader.into_memory_reader(offset.into(), jbk::End::new_size(sub_size))?;
            let written = file.write(reader.get_slice(jbk::Offset::zero(), jbk::End::None)?)?;
            offset += written;
            if offset == size {
                break;
            }
        }
        if self.print_progress {
            println!("{}", abs_path.display());
        }
        current_path.pop();
        Ok(())
    }
    fn on_link(&self, current_path: &mut PathBuf, link: &Link) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(link.path.clone()));
        if !self.should_extract(current_path, false) {
            current_path.pop();
            return Ok(());
        }
        let abs_path = self.abs_path(current_path);
        symlink(
            PathBuf::from(OsString::from_vec(link.target.clone())),
            PathBuf::from(&abs_path),
        )?;
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
    files_to_extract: HashSet<PathBuf>,
    progress: bool,
) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut walker = Walker::new(&arx, Default::default());
    let extractor = Extractor {
        arx: &arx,
        files: files_to_extract,
        base_dir: outdir.to_path_buf(),
        print_progress: progress,
    };
    walker.run(&extractor)
}
