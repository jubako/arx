use jbk::reader::MayMissPack;
use libarx::{Arx, ArxError, ArxFormatError, CommonEntry, FileEntry};
use std::path::PathBuf;

pub struct ArxModel {
    pub archive: Arx,
    pub path: PathBuf,
    pub roots: Vec<(jbk::EntryRange, String)>,
    pub entry_list: Vec<libarx::FullEntry>,
}

impl ArxModel {
    pub fn open(path: PathBuf) -> Result<Self, libarx::ArxError> {
        let archive = Arx::new(&path)?;

        let mut s = Self {
            archive,
            path,
            roots: vec![],
            entry_list: vec![],
        };
        s.update_entries()?;
        Ok(s)
    }

    pub fn root(&self) -> Option<(jbk::EntryRange, String)> {
        self.roots.last().cloned()
    }

    pub fn enter_in(&mut self, new_root: (jbk::EntryRange, String)) -> Result<(), ArxError> {
        self.roots.push(new_root);
        self.update_entries()
    }

    pub fn jump_off(&mut self, index: usize) -> Result<(), ArxError> {
        let _ = self.roots.split_off(index);
        self.update_entries()
    }

    fn update_entries(&mut self) -> Result<(), libarx::ArxError> {
        let builder = libarx::RealBuilder::<libarx::FullBuilder>::new(&self.archive.properties);
        let read_entry = match self.root() {
            None => libarx::ReadEntry::new(&self.archive.root_index, &builder),
            Some(r) => libarx::ReadEntry::new(&r.0, &builder),
        };
        self.entry_list = read_entry.collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    pub fn extract(
        &self,
        f: FileEntry,
        outfile: &std::path::Path,
    ) -> Result<bool, libarx::ArxError> {
        let bytes = self
            .archive
            .get_bytes(f.content())?
            .and_then(|m| m.transpose())
            .ok_or(ArxFormatError(
                "Entry Content should point to valid content",
            ))?;
        match bytes {
            MayMissPack::FOUND(bytes) => {
                use std::io::Write;

                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(outfile)?;
                let size = bytes.size().into_u64();
                let mut offset = 0;
                loop {
                    let sub_size = std::cmp::min(size - offset, 4 * 1024) as usize;
                    let written = file.write(&bytes.get_slice(offset.into(), sub_size)?)?;
                    offset += written as u64;
                    if offset == size {
                        break;
                    }
                }
                Ok(true)
            }
            MayMissPack::MISSING(_) => Ok(false),
        }
    }

    pub fn extract_and_open(&self, f: FileEntry) -> Result<(), libarx::ArxError> {
        let tmpdir = tempfile::tempdir()?;
        let tmp_file = tmpdir
            .keep()
            .join(String::from_utf8_lossy(f.path()).as_ref());

        if self.extract(f, &tmp_file)? {
            open::that_detached(tmp_file)?;
        }
        Ok(())
    }
}
