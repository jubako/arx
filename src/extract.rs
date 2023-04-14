use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::ffi::OsString;
use std::fs::{create_dir, create_dir_all, File};
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
}

impl libarx::walk::Operator<PathBuf, (FileEntry, Link, Path)> for Extractor<'_> {
    fn on_start(&self, current_path: &mut PathBuf) -> jbk::Result<()> {
        create_dir_all(current_path)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, path: &Path) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(path.clone()));
        create_dir(current_path)?;
        Ok(())
    }
    fn on_directory_exit(&self, current_path: &mut PathBuf, _path: &Path) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut PathBuf, entry: &FileEntry) -> jbk::Result<()> {
        let reader = self.arx.container.get_reader(entry.content)?;
        current_path.push(OsString::from_vec(entry.path.clone()));
        let mut file = File::create(&PathBuf::from(&*current_path))?;
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
        current_path.pop();
        Ok(())
    }
    fn on_link(&self, current_path: &mut PathBuf, link: &Link) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(link.path.clone()));
        symlink(
            PathBuf::from(OsString::from_vec(link.target.clone())),
            PathBuf::from(&*current_path),
        )?;
        current_path.pop();
        Ok(())
    }
}

pub fn extract<INP, OUTP>(infile: INP, outdir: OUTP) -> jbk::Result<()>
where
    INP: AsRef<std::path::Path>,
    PathBuf: From<OUTP>,
{
    let arx = libarx::Arx::new(infile)?;
    let index = arx.get_index_for_name("arx_root")?;
    let mut walker = libarx::walk::Walker::new(&arx, outdir.into());
    walker.run::<FullBuilder>(index, &Extractor { arx: &arx })
}
