use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

type Path = Vec<u8>;

struct EntryBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl libarx::walk::Builder for EntryBuilder {
    type Entry = Path;

    fn new(properties: &libarx::AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(
        &self,
        _idx: jbk::EntryIdx,
        reader: &libarx::walk::Reader,
    ) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

struct Lister {}

impl libarx::walk::Operator<libarx::LightPath, Path, Path, Path> for Lister {
    fn on_start(&self, _current_path: &mut libarx::LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut libarx::LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_directory_enter(
        &self,
        current_path: &mut libarx::LightPath,
        path: &Path,
    ) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(path.clone()));
        Ok(current_path.println()?)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut libarx::LightPath,
        _path: &Path,
    ) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut libarx::LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
    fn on_link(&self, current_path: &mut libarx::LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
}

pub fn list<P: AsRef<std::path::Path>>(infile: P) -> jbk::Result<()> {
    let arx = libarx::Arx::new(infile)?;
    let index = arx.get_index_for_name("arx_root")?;
    let mut walker = libarx::walk::Walker::new(&arx, Default::default());
    walker.run::<EntryBuilder, EntryBuilder, EntryBuilder>(index, &Lister {})
}
