use crate::common::*;
use jubako as jbk;
use std::path::{Path, PathBuf};

struct Lister {}

impl ArxOperator for Lister {
    fn on_start(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }

    fn on_file(&self, current_path: &mut PathBuf, entry: &FileEntry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        println!("{}", current_path.display());
        current_path.pop();
        Ok(())
    }

    fn on_link(&self, current_path: &mut PathBuf, entry: &LinkEntry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        println!("{}", current_path.display());
        current_path.pop();
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, entry: &DirEntry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        println!("{}", current_path.display());
        Ok(())
    }

    fn on_directory_exit(&self, current_path: &mut PathBuf, _entry: &DirEntry) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
}

pub fn list<P: AsRef<Path>>(infile: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = ArxRunner::new(&arx, PathBuf::with_capacity(2048));

    let op = Lister {};
    runner.run(arx.root_index()?, &op)
}
