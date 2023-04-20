use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{create_dir, create_dir_all, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

type Path = Vec<u8>;

struct FileEntry {
    path: Path,
    content: jbk::ContentAddress,
}

struct Link {
    path: Path,
    target: Path,
}

struct FileBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
}

impl libarx::Builder for FileBuilder {
    type Entry = FileEntry;

    fn new(properties: &libarx::AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            content_address_property: properties.file_content_address_property,
        }
    }

    fn create_entry(
        &self,
        _idx: jbk::EntryIdx,
        reader: &libarx::Reader,
    ) -> jbk::Result<Self::Entry> {
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

impl libarx::Builder for LinkBuilder {
    type Entry = Link;

    fn new(properties: &libarx::AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
            link_property: properties.link_target_property.clone(),
        }
    }

    fn create_entry(
        &self,
        _idx: jbk::EntryIdx,
        reader: &libarx::Reader,
    ) -> jbk::Result<Self::Entry> {
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

impl libarx::Builder for DirBuilder {
    type Entry = Path;

    fn new(properties: &libarx::AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(
        &self,
        _idx: jbk::EntryIdx,
        reader: &libarx::Reader,
    ) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

type FullBuilder = (FileBuilder, LinkBuilder, DirBuilder);

struct Extractor<'a> {
    arx: &'a libarx::Arx,
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

impl libarx::walk::Operator<PathBuf, FullBuilder> for Extractor<'_> {
    fn on_start(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, path: &Path) -> jbk::Result<bool> {
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
    fn on_directory_exit(&self, current_path: &mut PathBuf, _path: &Path) -> jbk::Result<()> {
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

pub fn extract<INP, OUTP>(
    infile: INP,
    outdir: OUTP,
    extract_files: Vec<PathBuf>,
    progress: bool,
) -> jbk::Result<()>
where
    INP: AsRef<std::path::Path>,
    PathBuf: From<OUTP>,
{
    let arx = libarx::Arx::new(infile)?;
    let mut walker = libarx::walk::Walker::new(&arx, Default::default());
    let extractor = Extractor {
        arx: &arx,
        files: extract_files.into_iter().collect(),
        base_dir: outdir.into(),
        print_progress: progress,
    };
    walker.run(&extractor)
}
