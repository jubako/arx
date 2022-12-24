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
    content_address: jbk::reader::Content,
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

    pub fn get_content_address(&self) -> &jbk::reader::Content {
        &self.content_address
    }
}

struct FileBuilder {
    path: jbk::reader::builder::ArrayProperty,
    parent: jbk::reader::builder::IntProperty,
    content_address: jbk::reader::builder::ContentProperty,
}

impl FileBuilder {
    pub fn create(
        &self,
        idx: jbk::EntryIdx,
        reader: &jbk::Reader,
        resolver: jbk::reader::Resolver,
    ) -> jbk::Result<FileEntry> {
        let path = self.path.create(reader)?;
        let parent = (self.parent.create(reader)? as u32).into();
        let content_address = self.content_address.create(reader)?;
        Ok(FileEntry {
            idx,
            path,
            parent,
            content_address,
            resolver,
        })
    }

    pub fn new_from_layout(layout: &jbk::reader::layout::Variant) -> jbk::Result<Self> {
        let path = (&layout.properties[0]).try_into()?;
        let parent = (&layout.properties[1]).try_into()?;
        let content_address = (&layout.properties[2]).try_into()?;
        Ok(Self {
            path,
            parent,
            content_address,
        })
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

struct DirBuilder {
    path: jbk::reader::builder::ArrayProperty,
    parent: jbk::reader::builder::IntProperty,
    first_child: jbk::reader::builder::IntProperty,
    nb_children: jbk::reader::builder::IntProperty,
}

impl DirBuilder {
    pub fn create(
        &self,
        idx: jbk::EntryIdx,
        reader: &jbk::Reader,
        resolver: jbk::reader::Resolver,
    ) -> jbk::Result<DirEntry> {
        let path = self.path.create(reader)?;
        let parent = (self.parent.create(reader)? as u32).into();
        let first_child = (self.first_child.create(reader)? as u32).into();
        let nb_children = (self.nb_children.create(reader)? as u32).into();
        Ok(DirEntry {
            idx,
            path,
            parent,
            first_child,
            nb_children,
            resolver,
        })
    }

    pub fn new_from_layout(layout: &jbk::reader::layout::Variant) -> jbk::Result<Self> {
        let path = (&layout.properties[0]).try_into()?;
        let parent = (&layout.properties[1]).try_into()?;
        let first_child = (&layout.properties[2]).try_into()?;
        let nb_children = (&layout.properties[3]).try_into()?;
        Ok(Self {
            path,
            parent,
            first_child,
            nb_children,
        })
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

struct LinkBuilder {
    path: jbk::reader::builder::ArrayProperty,
    parent: jbk::reader::builder::IntProperty,
    target_link: jbk::reader::builder::ArrayProperty,
}

impl LinkBuilder {
    pub fn create(
        &self,
        idx: jbk::EntryIdx,
        reader: &jbk::Reader,
        resolver: jbk::reader::Resolver,
    ) -> jbk::Result<LinkEntry> {
        let path = self.path.create(reader)?;
        let parent = (self.parent.create(reader)? as u32).into();
        let target = self.target_link.create(reader)?;
        Ok(LinkEntry {
            idx,
            path,
            parent,
            target,
            resolver,
        })
    }

    pub fn new_from_layout(layout: &jbk::reader::layout::Variant) -> jbk::Result<Self> {
        let path = (&layout.properties[0]).try_into()?;
        let parent = (&layout.properties[1]).try_into()?;
        let target_link = (&layout.properties[2]).try_into()?;
        Ok(Self {
            path,
            parent,
            target_link,
        })
    }
}

pub struct Builder {
    value_storage: Rc<jbk::reader::ValueStorage>,
    variant_id: jbk::reader::builder::Property<u8>,
    file_builder: FileBuilder,
    dir_builder: DirBuilder,
    link_builder: LinkBuilder,
}

impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx, reader: &jbk::Reader) -> jbk::Result<Self::Entry> {
        let resolver = jbk::reader::Resolver::new(Rc::clone(&self.value_storage));
        Ok(match self.variant_id.create(reader)? {
            0 => Entry::File(self.file_builder.create(idx, reader, resolver)?),
            1 => Entry::Dir(self.dir_builder.create(idx, reader, resolver)?),
            2 => Entry::Link(self.link_builder.create(idx, reader, resolver)?),
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
    fn check_layout(&self, layout: &jbk::reader::layout::Layout) -> jbk::Result<Self::Builder> {
        assert_eq!(layout.variants.len(), 3);
        let file_builder = FileBuilder::new_from_layout(&layout.variants[0])?;
        let dir_builder = DirBuilder::new_from_layout(&layout.variants[1])?;
        let link_builder = LinkBuilder::new_from_layout(&layout.variants[2])?;
        Ok(Builder {
            value_storage: Rc::clone(&self.value_storage),
            variant_id: jbk::reader::builder::Property::new(jbk::Offset::zero()),
            file_builder,
            dir_builder,
            link_builder,
        })
    }
}

pub struct EntryCompare {
    resolver: Rc<jbk::reader::Resolver>,
    path_value: Vec<u8>,
}

impl EntryCompare {
    pub fn new(resolver: Rc<jbk::reader::Resolver>, component: &OsStr) -> Self {
        let path_value = component.to_os_string().into_vec();
        Self {
            resolver,
            path_value,
        }
    }
}

impl jbk::reader::CompareTrait<Entry> for EntryCompare {
    fn compare(&self, e: &Entry) -> jbk::Result<std::cmp::Ordering> {
        match e {
            Entry::Dir(e) => self.resolver.compare_array(&e.path, &self.path_value),
            Entry::File(e) => self.resolver.compare_array(&e.path, &self.path_value),
            Entry::Link(e) => self.resolver.compare_array(&e.path, &self.path_value),
        }
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

    pub fn walk<'s, 'finder>(
        &'s self,
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

    fn _run<'s>(
        &'s mut self,
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
                        finder.get_store(),
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
