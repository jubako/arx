use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub enum Entry {
    File(FileEntry),
    Dir(DirEntry),
    Link(LinkEntry),
}

impl Entry {
    pub fn idx(&self) -> jbk::EntryIdx {
        match self {
            Self::File(e) => e.idx,
            Self::Dir(e) => e.idx,
            Self::Link(e) => e.idx,
        }
    }

    pub fn get_parent(&self) -> Option<jbk::EntryIdx> {
        match self {
            Self::File(e) => e.get_parent(),
            Self::Dir(e) => e.get_parent(),
            Self::Link(e) => e.get_parent(),
        }
    }

    pub fn get_path(&self) -> jbk::Result<String> {
        match self {
            Self::File(e) => e.get_path(),
            Self::Dir(e) => e.get_path(),
            Self::Link(e) => e.get_path(),
        }
    }
}

pub struct FileEntry {
    idx: jbk::EntryIdx,
    path: jbk::reader::Array,
    parent: jbk::EntryIdx,
    content_address: jbk::reader::ContentAddress,
    resolver: jbk::reader::Resolver,
}

impl FileEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.resolver.resolve_array_to_vec(&self.path, &mut path)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_parent(&self) -> Option<jbk::EntryIdx> {
        if !self.parent {
            None
        } else {
            Some(self.parent - 1)
        }
    }

    pub fn get_content_address(&self) -> jbk::reader::ContentAddress {
        self.content_address
    }
}

pub struct DirEntry {
    idx: jbk::EntryIdx,
    path: jbk::reader::Array,
    parent: jbk::EntryIdx,
    first_child: jbk::EntryIdx,
    nb_children: jbk::EntryCount,
    resolver: jbk::reader::Resolver,
}

impl DirEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.resolver.resolve_array_to_vec(&self.path, &mut path)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_parent(&self) -> Option<jbk::EntryIdx> {
        if !self.parent {
            None
        } else {
            Some(self.parent - 1)
        }
    }

    pub fn get_first_child(&self) -> jbk::EntryIdx {
        self.first_child
    }

    pub fn get_nb_children(&self) -> jbk::EntryCount {
        self.nb_children
    }
}

pub struct LinkEntry {
    idx: jbk::EntryIdx,
    path: jbk::reader::Array,
    parent: jbk::EntryIdx,
    target: jbk::reader::Array,
    resolver: jbk::reader::Resolver,
}

impl LinkEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.resolver.resolve_array_to_vec(&self.path, &mut path)?;
        Ok(String::from_utf8(path)?)
    }

    pub fn get_parent(&self) -> Option<jbk::EntryIdx> {
        if !self.parent {
            None
        } else {
            Some(self.parent - 1)
        }
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.resolver
            .resolve_array_to_vec(&self.target, &mut path)?;
        Ok(String::from_utf8(path)?)
    }
}

pub struct Builder {
    value_storage: Rc<jbk::reader::ValueStorage>,
    store: Rc<jbk::reader::EntryStore>,
    path_property: jbk::reader::builder::ArrayProperty,
    parent_property: jbk::reader::builder::IntProperty,
    variant_id_property: jbk::reader::builder::Property<u8>,
    file_content_address_property: jbk::reader::builder::ContentProperty,
    dir_first_child_property: jbk::reader::builder::IntProperty,
    dir_nb_children_property: jbk::reader::builder::IntProperty,
    link_target_property: jbk::reader::builder::ArrayProperty,
}

impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let resolver = jbk::reader::Resolver::new(Rc::clone(&self.value_storage));
        let reader = self.store.get_entry_reader(idx);
        let path = self.path_property.create(&reader)?;
        let parent = (self.parent_property.create(&reader)? as u32).into();
        Ok(match self.variant_id_property.create(&reader)? {
            0 => {
                let content_address = self.file_content_address_property.create(&reader)?;
                Entry::File(FileEntry {
                    idx,
                    path,
                    parent,
                    content_address,
                    resolver,
                })
            }
            1 => {
                let first_child = (self.dir_first_child_property.create(&reader)? as u32).into();
                let nb_children = (self.dir_nb_children_property.create(&reader)? as u32).into();
                Entry::Dir(DirEntry {
                    idx,
                    path,
                    parent,
                    first_child,
                    nb_children,
                    resolver,
                })
            }
            2 => {
                let target = self.link_target_property.create(&reader)?;
                Entry::Link(LinkEntry {
                    idx,
                    path,
                    parent,
                    target,
                    resolver,
                })
            }
            _ => unreachable!(),
        })
    }
}

pub struct Schema {
    value_storage: Rc<jbk::reader::ValueStorage>,
}

impl Schema {
    pub fn new(container: &jbk::reader::Container) -> Self {
        Self {
            value_storage: Rc::clone(container.get_value_storage()),
        }
    }
}

impl jbk::reader::schema::SchemaTrait for Schema {
    type Builder = Builder;
    fn create_builder(&self, store: Rc<jbk::reader::EntryStore>) -> jbk::Result<Rc<Self::Builder>> {
        let layout = store.layout();
        let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 3);
        let path_property = (&layout.common[0]).try_into()?;
        let parent_property = (&layout.common[1]).try_into()?;
        let variant_id_property = jbk::reader::builder::Property::new(*variant_offset);
        let file_content_address_property = (&variants[0][0]).try_into()?;
        let dir_first_child_property = (&variants[1][0]).try_into()?;
        let dir_nb_children_property = (&variants[1][1]).try_into()?;
        let link_target_property = (&variants[2][0]).try_into()?;
        Ok(Rc::new(Builder {
            value_storage: Rc::clone(&self.value_storage),
            store,
            path_property,
            parent_property,
            variant_id_property,
            file_content_address_property,
            dir_first_child_property,
            dir_nb_children_property,
            link_target_property,
        }))
    }
}

pub struct EntryCompare<'resolver, 'builder> {
    resolver: &'resolver jbk::reader::Resolver,
    builder: &'builder Builder,
    path_value: Vec<u8>,
}

impl<'resolver, 'builder> EntryCompare<'resolver, 'builder> {
    pub fn new(
        resolver: &'resolver jbk::reader::Resolver,
        builder: &'builder Builder,
        component: &OsStr,
    ) -> Self {
        let path_value = component.to_os_string().into_vec();
        Self {
            resolver,
            builder,
            path_value,
        }
    }
}

impl jbk::reader::CompareTrait<Schema> for EntryCompare<'_, '_> {
    fn compare_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<std::cmp::Ordering> {
        let reader = self.builder.store.get_entry_reader(idx);
        let entry_path = self.builder.path_property.create(&reader)?;
        self.resolver.compare_array(&entry_path, &self.path_value)
    }
}

pub struct ReadEntry<'finder> {
    finder: &'finder jbk::reader::Finder<Schema>,
    current: jbk::EntryIdx,
    end: jbk::EntryIdx,
}

impl<'finder> ReadEntry<'finder> {
    pub fn new(finder: &'finder jbk::reader::Finder<Schema>) -> Self {
        let end = jbk::EntryIdx::from(0) + finder.count();
        Self {
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
            self.current += 1;
            Some(entry)
        }
    }
}

pub trait ArxOperator {
    fn on_start(&self, current_path: &mut PathBuf) -> jbk::Result<()>;
    fn on_stop(&self, current_path: &mut PathBuf) -> jbk::Result<()>;
    fn on_directory_enter(&self, current_path: &mut PathBuf, entry: &DirEntry) -> jbk::Result<()>;
    fn on_directory_exit(&self, current_path: &mut PathBuf, entry: &DirEntry) -> jbk::Result<()>;
    fn on_file(&self, current_path: &mut PathBuf, entry: &FileEntry) -> jbk::Result<()>;
    fn on_link(&self, current_path: &mut PathBuf, entry: &LinkEntry) -> jbk::Result<()>;
}

pub struct Arx {
    container: jbk::reader::Container,
    pub schema: Schema,
}

impl std::ops::Deref for Arx {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl Arx {
    pub fn new<P: AsRef<Path>>(file: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&file)?;
        let schema = Schema::new(&container);
        Ok(Self { container, schema })
    }

    pub fn walk<'finder>(
        &self,
        finder: &'finder jbk::reader::Finder<Schema>,
    ) -> ReadEntry<'finder> {
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

    pub fn run(
        &mut self,
        finder: jbk::reader::Finder<Schema>,
        op: &dyn ArxOperator,
    ) -> jbk::Result<()> {
        op.on_start(&mut self.current_path)?;
        self._run(finder, op)?;
        op.on_stop(&mut self.current_path)
    }

    fn _run(
        &mut self,
        finder: jbk::reader::Finder<Schema>,
        op: &dyn ArxOperator,
    ) -> jbk::Result<()> {
        for entry in self.arx.walk(&finder) {
            match entry? {
                Entry::File(e) => op.on_file(&mut self.current_path, &e)?,
                Entry::Link(e) => op.on_link(&mut self.current_path, &e)?,
                Entry::Dir(e) => {
                    op.on_directory_enter(&mut self.current_path, &e)?;
                    let finder = jbk::reader::Finder::new(
                        Rc::clone(finder.builder()),
                        e.get_first_child(),
                        e.get_nb_children(),
                    );
                    self._run(finder, op)?;
                    op.on_directory_exit(&mut self.current_path, &e)?;
                }
            }
        }
        Ok(())
    }
}
