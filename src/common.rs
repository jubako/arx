use jubako as jbk;
use jbk::reader::EntryTrait;
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
    idx: jbk::Idx<u32>,
    entry: jbk::reader::Entry,
    resolver: Rc<jbk::reader::Resolver>,
}

impl Entry {
    pub fn new(
        idx: jbk::Idx<u32>,
        entry: jbk::reader::Entry,
        resolver: Rc<jbk::reader::Resolver>,
    ) -> Self {
        Self {
            idx,
            entry,
            resolver,
        }
    }

    pub fn idx(&self) -> jbk::Idx<u32> {
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
            .resolve_to_vec(self.entry.get_value(0.into())?)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_parent(&self) -> Option<jbk::Idx<u32>> {
        let idx = self
            .resolver
            .resolve_to_unsigned(self.entry.get_value(1.into()).unwrap()) as u32;
        if idx == 0 {
            None
        } else {
            Some(jbk::Idx(idx - 1))
        }
    }

    pub fn get_content_address(&self) -> &jbk::reader::Content {
        assert!(self.is_file());
        self.resolver
            .resolve_to_content(self.entry.get_value(2.into()).unwrap())
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        assert!(self.is_link());
        let path = self
            .resolver
            .resolve_to_vec(self.entry.get_value(2.into())?)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_first_child(&self) -> jbk::Idx<u32> {
        assert!(self.is_dir());
        jbk::Idx(
            self.resolver
                .resolve_to_unsigned(self.entry.get_value(2.into()).unwrap()) as u32,
        )
    }

    pub fn get_nb_children(&self) -> jbk::Count<u32> {
        assert!(self.is_dir());
        jbk::Count(
            self.resolver
                .resolve_to_unsigned(self.entry.get_value(3.into()).unwrap()) as u32,
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

pub struct ReadEntry<'a> {
    finder: &'a jbk::reader::Finder,
    current: jbk::Idx<u32>,
    end: jbk::Idx<u32>,
}

impl<'a> ReadEntry<'a> {
    pub fn new(finder: &'a jbk::reader::Finder) -> Self {
        let end = jbk::Idx(0) + finder.count();
        Self {
            finder,
            current: jbk::Idx(0),
            end,
        }
    }

    pub fn skip(&mut self, to_skip: jbk::Count<u32>) {
        self.current += to_skip.0;
    }
}

impl<'a> Iterator for ReadEntry<'a> {
    type Item = jbk::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let entry = self.finder.get_entry(self.current);
            let ret = Some(match entry {
                Ok(e) => Ok(Entry::new(
                    self.current,
                    e,
                    Rc::clone(self.finder.get_resolver()),
                )),
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

    pub fn walk<'a>(&self, finder: &'a jbk::reader::Finder) -> ReadEntry<'a> {
        ReadEntry::new(finder)
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
                        Rc::clone(finder.get_resolver()),
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
