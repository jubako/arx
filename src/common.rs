use jbk::reader::EntryTrait;
use jubako as jbk;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Link,
}

pub struct Entry {
    idx: jbk::EntryIdx,
    entry: jbk::reader::Entry,
    resolver: jbk::reader::Resolver,
}

impl Entry {
    pub fn new(
        idx: jbk::EntryIdx,
        entry: jbk::reader::Entry,
        value_storage: Rc<jbk::reader::ValueStorage>,
    ) -> Self {
        let resolver = jbk::reader::Resolver::new(value_storage);
        Self {
            idx,
            entry,
            resolver,
        }
    }

    pub fn idx(&self) -> jbk::EntryIdx {
        self.idx
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
        let path = self
            .resolver
            .resolve_to_vec(&self.entry.get_value(0.into())?)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_parent(&self) -> Option<jbk::EntryIdx> {
        let idx = self
            .resolver
            .resolve_to_unsigned(&self.entry.get_value(1.into()).unwrap()) as u32;
        if idx == 0 {
            None
        } else {
            Some(jbk::EntryIdx::from(idx - 1))
        }
    }

    pub fn get_content_address(&self) -> jbk::reader::Content {
        assert!(self.is_file());
        self.resolver
            .resolve_to_content(&self.entry.get_value(2.into()).unwrap())
            .clone()
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        assert!(self.is_link());
        let path = self
            .resolver
            .resolve_to_vec(&self.entry.get_value(2.into())?)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_first_child(&self) -> jbk::EntryIdx {
        assert!(self.is_dir());
        jbk::EntryIdx::from(
            self.resolver
                .resolve_to_unsigned(&self.entry.get_value(2.into()).unwrap()) as u32,
        )
    }

    pub fn get_nb_children(&self) -> jbk::EntryCount {
        assert!(self.is_dir());
        jbk::EntryCount::from(
            self.resolver
                .resolve_to_unsigned(&self.entry.get_value(3.into()).unwrap()) as u32,
        )
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

pub struct ReadEntry<'finder> {
    value_storage: Rc<jbk::reader::ValueStorage>,
    finder: &'finder jbk::reader::Finder,
    current: jbk::EntryIdx,
    end: jbk::EntryIdx,
}

impl<'finder> ReadEntry<'finder> {
    pub fn new(
        value_storage: Rc<jbk::reader::ValueStorage>,
        finder: &'finder jbk::reader::Finder,
    ) -> Self {
        let end = jbk::EntryIdx::from(0) + finder.count();
        Self {
            value_storage,
            finder,
            current: jbk::EntryIdx::from(0),
            end,
        }
    }

    pub fn skip(&mut self, to_skip: jbk::EntryCount) {
        self.current += to_skip;
    }
}

impl<'finder> Iterator for ReadEntry<'finder> {
    type Item = jbk::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let entry = self.finder.get_entry(self.current);
            let ret = Some(match entry {
                Ok(e) => Ok(Entry::new(self.current, e, Rc::clone(&self.value_storage))),
                Err(e) => Err(e),
            });
            self.current += 1;
            ret
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

pub struct Arx(jbk::reader::Container);

impl std::ops::Deref for Arx {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Arx {
    pub fn new<P: AsRef<Path>>(file: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&file)?;
        Ok(Self(container))
    }

    pub fn walk<'s, 'finder>(&'s self, finder: &'finder jbk::reader::Finder) -> ReadEntry<'finder> {
        ReadEntry::new(Rc::clone(self.get_value_storage()), finder)
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

    pub fn run(&mut self, finder: jbk::reader::Finder, op: &dyn ArxOperator) -> jbk::Result<()> {
        op.on_start(&self.current_path)?;
        self._run(finder, op)?;
        op.on_stop(&self.current_path)
    }

    fn _run(&mut self, finder: jbk::reader::Finder, op: &dyn ArxOperator) -> jbk::Result<()> {
        for entry in self.arx.walk(&finder) {
            let entry = entry?;
            match entry.get_type() {
                EntryKind::File => op.on_file(&self.current_path, &entry)?,
                EntryKind::Link => op.on_link(&self.current_path, &entry)?,
                EntryKind::Directory => {
                    op.on_directory_enter(&self.current_path, &entry)?;
                    self.current_path.push(entry.get_path()?);
                    let finder = jbk::reader::Finder::new(
                        Rc::clone(finder.get_store()),
                        entry.get_first_child(),
                        entry.get_nb_children(),
                        finder.get_resolver().clone(),
                    );
                    self._run(finder, op)?;
                    self.current_path.pop();
                    op.on_directory_exit(&self.current_path, &entry)?;
                }
            }
        }
        Ok(())
    }
}
