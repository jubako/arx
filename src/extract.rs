use crate::common::*;
use jubako as jbk;
use std::fs::{create_dir, create_dir_all, File};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

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

    fn on_file(&self, current_path: &mut PathBuf, entry: &FileEntry) -> jbk::Result<()> {
        let reader = self.container.get_reader(entry.get_content_address())?;
        current_path.push(entry.get_path()?);
        let mut file = File::create(&current_path)?;
        let size = reader.size().into_usize();
        let mut offset = 0;
        loop {
            let sub_size = std::cmp::min(size - offset, 4 * 1024);
            let reader = reader
                .create_sub_memory_reader(offset.into(), jbk::End::new_size(sub_size))?
                .into_memory_reader()?;
            let written = file.write(reader.get_slice(jbk::Offset::zero(), jbk::End::None)?)?;
            offset += written;
            if offset == size {
                break;
            }
        }
        current_path.pop();
        Ok(())
    }

    fn on_link(&self, current_path: &mut PathBuf, entry: &LinkEntry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        let target = entry.get_target_link()?;
        symlink(target, &current_path)?;
        current_path.pop();
        Ok(())
    }

    fn on_directory_enter(&self, current_path: &mut PathBuf, entry: &DirEntry) -> jbk::Result<()> {
        current_path.push(entry.get_path()?);
        create_dir(&current_path)?;
        Ok(())
    }

    fn on_directory_exit(&self, current_path: &mut PathBuf, _entry: &DirEntry) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
}

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = ArxRunner::new(&arx, outdir.as_ref().to_path_buf());

    let op = Extractor::new(&arx);
    runner.run(arx.root_index()?, &op)
}
