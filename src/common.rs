use jbk::reader::builder::{BuilderTrait, PropertyBuilderTrait};
use jbk::reader::Range;
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
}

impl FileEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.path.resolve_to_vec(&mut path)?;
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
}

impl DirEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.path.resolve_to_vec(&mut path)?;
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

impl From<&DirEntry> for jbk::EntryRange {
    fn from(entry: &DirEntry) -> Self {
        Self::new(entry.get_first_child(), entry.get_nb_children())
    }
}

impl Range for DirEntry {
    fn offset(&self) -> jbk::EntryIdx {
        self.get_first_child()
    }

    fn count(&self) -> jbk::EntryCount {
        self.get_nb_children()
    }
}

pub struct LinkEntry {
    idx: jbk::EntryIdx,
    path: jbk::reader::Array,
    parent: jbk::EntryIdx,
    target: jbk::reader::Array,
}

impl LinkEntry {
    pub fn get_path(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.path.resolve_to_vec(&mut path)?;
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
        self.target.resolve_to_vec(&mut path)?;
        Ok(String::from_utf8(path)?)
    }
}

pub struct Builder {
    store: Rc<jbk::reader::EntryStore>,
    path_property: jbk::reader::builder::ArrayProperty,
    parent_property: jbk::reader::builder::IntProperty,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    file_content_address_property: jbk::reader::builder::ContentProperty,
    dir_first_child_property: jbk::reader::builder::IntProperty,
    dir_nb_children_property: jbk::reader::builder::IntProperty,
    link_target_property: jbk::reader::builder::ArrayProperty,
}

impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let path = self.path_property.create(&reader)?;
        let parent = (self.parent_property.create(&reader)? as u32).into();
        Ok(match self.variant_id_property.create(&reader)?.into_u8() {
            0 => {
                let content_address = self.file_content_address_property.create(&reader)?;
                Entry::File(FileEntry {
                    idx,
                    path,
                    parent,
                    content_address,
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
                })
            }
            2 => {
                let target = self.link_target_property.create(&reader)?;
                Entry::Link(LinkEntry {
                    idx,
                    path,
                    parent,
                    target,
                })
            }
            _ => unreachable!(),
        })
    }
}

fn create_builder(
    store: Rc<jbk::reader::EntryStore>,
    value_storage: &jbk::reader::ValueStorage,
) -> jbk::Result<Builder> {
    let layout = store.layout();
    let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
    assert_eq!(variants.len(), 3);
    let path_property = (&layout.common[0], value_storage).try_into()?;
    let parent_property = (&layout.common[1], value_storage).try_into()?;
    let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_offset);
    let file_content_address_property = (&variants[0][0]).try_into()?;
    let dir_first_child_property = (&variants[1][0], value_storage).try_into()?;
    let dir_nb_children_property = (&variants[1][1], value_storage).try_into()?;
    let link_target_property = (&variants[2][0], value_storage).try_into()?;
    Ok(Builder {
        store,
        path_property,
        parent_property,
        variant_id_property,
        file_content_address_property,
        dir_first_child_property,
        dir_nb_children_property,
        link_target_property,
    })
}

pub struct EntryCompare<'builder> {
    builder: &'builder Builder,
    path_value: Vec<u8>,
}

impl<'builder> EntryCompare<'builder> {
    pub fn new(builder: &'builder Builder, component: &OsStr) -> Self {
        let path_value = component.to_os_string().into_vec();
        Self {
            builder,
            path_value,
        }
    }
}

impl jbk::reader::CompareTrait for EntryCompare<'_> {
    fn compare_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<std::cmp::Ordering> {
        let reader = self.builder.store.get_entry_reader(idx);
        let entry_path = self.builder.path_property.create(&reader)?;
        match entry_path.partial_cmp(&self.path_value) {
            Some(c) => Ok(c),
            None => Err("Cannot compare".into()),
        }
    }
}

pub struct ReadEntry<'builder> {
    builder: &'builder Builder,
    current: jbk::EntryIdx,
    end: jbk::EntryIdx,
}

impl<'builder> ReadEntry<'builder> {
    pub fn new<R: Range>(range: &R, builder: &'builder Builder) -> Self {
        let end = range.offset() + range.count();
        Self {
            builder,
            current: range.offset(),
            end,
        }
    }

    pub fn skip(&mut self, to_skip: jbk::EntryCount) {
        self.current += to_skip;
    }
}

impl<'builder> Iterator for ReadEntry<'builder> {
    type Item = jbk::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let entry = self.builder.create_entry(self.current);
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
    pub container: jbk::reader::Container,
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
        Ok(Self { container })
    }

    pub fn create_builder(&self, index: &jbk::reader::Index) -> jbk::Result<Builder> {
        create_builder(
            index.get_store(self.get_entry_storage())?,
            self.get_value_storage(),
        )
    }

    pub fn root_index(&self) -> jbk::Result<jbk::reader::Index> {
        let directory = self.container.get_directory_pack();
        directory.get_index_from_name("arx_root")
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

    pub fn run(&mut self, index: jbk::reader::Index, op: &dyn ArxOperator) -> jbk::Result<()> {
        let builder = self.arx.create_builder(&index)?;
        op.on_start(&mut self.current_path)?;
        self._run(&index, &builder, op)?;
        op.on_stop(&mut self.current_path)
    }

    fn _run<R: Range>(
        &mut self,
        range: &R,
        builder: &Builder,
        op: &dyn ArxOperator,
    ) -> jbk::Result<()> {
        let read_entry = ReadEntry::new(range, builder);
        for entry in read_entry {
            match entry? {
                Entry::File(e) => op.on_file(&mut self.current_path, &e)?,
                Entry::Link(e) => op.on_link(&mut self.current_path, &e)?,
                Entry::Dir(e) => {
                    op.on_directory_enter(&mut self.current_path, &e)?;
                    self._run(&e, builder, op)?;
                    op.on_directory_exit(&mut self.current_path, &e)?;
                }
            }
        }
        Ok(())
    }
}
