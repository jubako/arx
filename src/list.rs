use crate::common::*;
use jubako as jbk;
use std::path::{Path, PathBuf};

struct Lister {}

impl ArxOperator for Lister {
    fn on_start(&self, _current_path: &dyn AsRef<Path>) -> jbk::Result<()> {
        Ok(())
    }

    fn on_stop(&self, _current_path: &dyn AsRef<Path>) -> jbk::Result<()> {
        Ok(())
    }

    fn on_file(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        println!("{}", path.display());
        Ok(())
    }

    fn on_link(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        println!("{}", path.display());
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        println!("{}", path.display());
        Ok(())
    }

    fn on_directory_exit(
        &self,
        _current_path: &dyn AsRef<Path>,
        _entry: &Entry,
    ) -> jbk::Result<()> {
        Ok(())
    }
}

pub fn list<P: AsRef<Path>>(infile: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = ArxRunner::new(&arx, PathBuf::new());

    let index = arx.directory.get_index_from_name("root")?;
    let resolver = arx.directory.get_resolver();
    let op = Lister {};
    runner.run(index.get_finder(resolver), &op)
}
