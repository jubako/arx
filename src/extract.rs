use crate::common::*;
use jubako as jbk;
use std::fs::{create_dir, create_dir_all, File};
use std::os::unix::fs::symlink;
use std::path::Path;

struct Extractor<'a> {
    container: &'a jbk::reader::Container,
}

impl<'a> Extractor<'a> {
    pub fn new(container: &'a jbk::reader::Container) -> Self {
        Self { container }
    }
}

impl<'a> ArxOperator for Extractor<'a> {
    fn on_start(&self, current_path: &dyn AsRef<Path>) -> jbk::Result<()> {
        create_dir_all(current_path)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &dyn AsRef<Path>) -> jbk::Result<()> {
        Ok(())
    }

    fn on_file(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        let content_address = entry.get_content_address();
        let reader = self.container.get_reader(content_address)?;
        let mut file = File::create(path)?;
        std::io::copy(&mut reader.create_stream_all(), &mut file)?;
        Ok(())
    }

    fn on_link(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        let target = entry.get_target_link()?;
        symlink(target, path)?;
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()> {
        let path = current_path.as_ref().join(entry.get_path()?);
        create_dir(&path)?;
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

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = ArxRunner::new(&arx, outdir.as_ref().to_path_buf());

    let index = arx.directory.get_index_from_name("root")?;
    let resolver = arx.directory.get_resolver();
    let op = Extractor::new(&arx.container);
    runner.run(index.get_finder(resolver), &op)
}
