use crate::common::*;
use jubako as jbk;
use std::fs::{create_dir, create_dir_all, File};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct Extractor<'a> {
    container: &'a jbk::reader::Container,
}

impl<'a> Extractor<'a> {
    pub fn new(container: &'a jbk::reader::Container) -> Self {
        Self { container }
    }
}

impl<'a> ArxOperator for Extractor<'a> {
    fn on_start(&self, current_path: &mut PathBuf) -> jbk::Result<()> {
        create_dir_all(current_path)?;
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_file(&self, current_path: &mut PathBuf, entry: &Entry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        let content_address = entry.get_content_address();
        let reader = self.container.get_reader(&content_address)?;
        let mut file = File::create(&current_path)?;
        std::io::copy(&mut reader.create_stream_all(), &mut file)?;
        current_path.pop();
        Ok(())
    }

    fn on_link(&self, current_path: &mut PathBuf, entry: &Entry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        let target = entry.get_target_link()?;
        symlink(target, &current_path)?;
        current_path.pop();
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, entry: &Entry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        create_dir(&current_path)?;
        Ok(())
    }

    fn on_directory_exit(&self, current_path: &mut PathBuf, _entry: &Entry) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
}

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = ArxRunner::new(&arx, outdir.as_ref().to_path_buf());

    let index = arx.get_index_for_name("root")?;
    let resolver = jbk::reader::Resolver::new(Rc::clone(arx.get_value_storage()));
    let op = Extractor::new(&arx);
    runner.run(index.get_finder(arx.get_entry_storage(), resolver)?, &op)
}
