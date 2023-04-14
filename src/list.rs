use crate::light_path::LightPath;
use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

type Path = Vec<u8>;

struct PathBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl libarx::Builder for PathBuilder {
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

type FullBuilder = (PathBuilder, PathBuilder, PathBuilder);

struct Lister {}

impl libarx::walk::Operator<LightPath, FullBuilder> for Lister {
    fn on_start(&self, _current_path: &mut LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_directory_enter(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(path.clone()));
        Ok(current_path.println()?)
    }
    fn on_directory_exit(&self, current_path: &mut LightPath, _path: &Path) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
    fn on_link(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
}

pub fn list<P: AsRef<std::path::Path>>(infile: P) -> jbk::Result<()> {
    let arx = libarx::Arx::new(infile)?;
    let mut walker = libarx::walk::Walker::new(&arx, Default::default());
    walker.run(&Lister {})
}
