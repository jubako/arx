use jubako as jbk;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(PartialEq)]
pub enum EntryKind {
    File,
    Directory,
    Link,
}

pub struct Entry {
    entry: jbk::reader::Entry,
    key_storage: Rc<jbk::reader::KeyStorage>,
}

impl Entry {
    pub fn new(entry: jbk::reader::Entry, key_storage: Rc<jbk::reader::KeyStorage>) -> Self {
        Self { entry, key_storage }
    }
    pub fn get_type(&self) -> EntryKind {
        match self.entry.get_variant_id() {
            0 => EntryKind::File,
            1 => EntryKind::Directory,
            2 => EntryKind::Link,
            _ => unreachable!(),
        }
    }

    pub fn is_file(&self) -> bool {
        self.entry.get_variant_id() == 0
    }

    pub fn is_dir(&self) -> bool {
        self.entry.get_variant_id() == 1
    }

    pub fn is_link(&self) -> bool {
        self.entry.get_variant_id() == 2
    }

    pub fn get_path(&self) -> jbk::Result<String> {
        if let jbk::reader::Value::Array(path) = self.entry.get_value(0.into()).unwrap() {
            let path = path.resolve_to_vec(&self.key_storage)?;
            Ok(String::from_utf8(path)?)
        } else {
            panic!()
        }
    }

    pub fn get_parent(&self) -> Option<jbk::Idx<u32>> {
        let v = self.entry.get_value(1.into()).unwrap();
        let idx = match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        };
        if idx == 0 {
            None
        } else {
            Some(jbk::Idx(idx - 1))
        }
    }

    pub fn get_content_address(&self) -> &jbk::reader::Content {
        assert!(self.is_file());
        let v = self.entry.get_value(2.into()).unwrap();
        if let jbk::reader::Value::Content(c) = v {
            c
        } else {
            panic!()
        }
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        assert!(self.is_link());
        let v = self.entry.get_value(2.into()).unwrap();
        if let jbk::reader::Value::Array(target) = v {
            let target = target.resolve_to_vec(&self.key_storage)?;
            Ok(String::from_utf8(target)?)
        } else {
            panic!()
        }
    }

    pub fn get_first_child(&self) -> jbk::Idx<u32> {
        assert!(self.is_dir());
        let v = self.entry.get_value(2.into()).unwrap();
        jbk::Idx(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        })
    }

    pub fn get_nb_children(&self) -> jbk::Count<u32> {
        assert!(self.is_dir());
        let v = self.entry.get_value(3.into()).unwrap();
        jbk::Count(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        })
    }

    pub fn as_range(&self) -> EntryRange {
        assert!(self.is_dir());
        EntryRange {
            start: self.get_first_child(),
            end: self.get_first_child() + self.get_nb_children(),
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            EntryKind::Directory => write!(f, "{}/", self.get_path().unwrap()),
            _ => write!(f, "{}", self.get_path().unwrap()),
        }
        //write!(f, "{}", self.get_path().or(Err(fmt::Error))?)
    }
}

pub struct EntryRange {
    pub start: jbk::Idx<u32>,
    pub end: jbk::Idx<u32>,
}

impl From<&jbk::reader::Index> for EntryRange {
    fn from(index: &jbk::reader::Index) -> Self {
        Self {
            start: index.entry_offset(),
            end: index.entry_offset() + index.entry_count(),
        }
    }
}

pub struct ReadEntry {
    index: jbk::reader::Index,
    key_storage: Rc<jbk::reader::KeyStorage>,
    current: jbk::Idx<u32>,
    end: jbk::Idx<u32>,
}

impl ReadEntry {
    pub fn new(directory: &Rc<jbk::reader::DirectoryPack>, range: EntryRange) -> jbk::Result<Self> {
        let index = directory.get_index_from_name("entries")?;
        let key_storage = directory.get_key_storage();
        Ok(Self {
            index,
            key_storage,
            current: range.start,
            end: range.end,
        })
    }
}

impl Iterator for ReadEntry {
    type Item = jbk::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let entry = self.index.get_entry(self.current);
            self.current += 1;
            Some(match entry {
                Ok(e) => Ok(Entry::new(e, Rc::clone(&self.key_storage))),
                Err(e) => Err(e),
            })
        }
    }
}

pub trait ArxOperator {
    fn on_start(&self, current_path: &dyn AsRef<Path>) -> jbk::Result<()>;
    fn on_stop(&self, current_path: &dyn AsRef<Path>) -> jbk::Result<()>;
    fn on_directory_enter(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()>;
    fn on_directory_exit(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()>;
    fn on_file(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()>;
    fn on_link(&self, current_path: &dyn AsRef<Path>, entry: &Entry) -> jbk::Result<()>;
}

pub struct Arx {
    pub container: jbk::reader::Container,
    pub directory: Rc<jbk::reader::DirectoryPack>,
}

impl Arx {
    pub fn new<P: AsRef<Path>>(file: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&file)?;
        let directory = Rc::clone(container.get_directory_pack()?);
        Ok(Self {
            container,
            directory,
        })
    }

    pub fn walk(&self, range: EntryRange) -> jbk::Result<ReadEntry> {
        ReadEntry::new(&self.directory, range)
    }
}

pub struct ArxRunner<'a> {
    arx: &'a Arx,
    current_path: PathBuf,
}

impl<'a> ArxRunner<'a> {
    pub fn new(arx: &'a Arx, current_path: PathBuf) -> Self {
        Self { arx, current_path }
    }

    pub fn run(&mut self, range: EntryRange, op: &dyn ArxOperator) -> jbk::Result<()> {
        op.on_start(&self.current_path)?;
        self._run(range, op)?;
        op.on_stop(&self.current_path)
    }

    fn _run(&mut self, range: EntryRange, op: &dyn ArxOperator) -> jbk::Result<()> {
        for entry in self.arx.walk(range)? {
            let entry = entry?;
            match entry.get_type() {
                EntryKind::File => op.on_file(&self.current_path, &entry)?,
                EntryKind::Link => op.on_link(&self.current_path, &entry)?,
                EntryKind::Directory => {
                    op.on_directory_enter(&self.current_path, &entry)?;
                    self.current_path.push(entry.get_path()?);
                    self._run(entry.as_range(), op)?;
                    self.current_path.pop();
                    op.on_directory_exit(&self.current_path, &entry)?;
                }
            }
        }
        Ok(())
    }
}
