mod entry_type;
mod light_path;
mod properties;

pub use entry_type::EntryType;
use jbk::reader::builder::{BuilderTrait, PropertyBuilderTrait};
use jbk::reader::Range;
use jubako as jbk;
pub use light_path::LightPath;
pub use properties::AllProperties;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

pub type EntryResult<T> = Result<T, EntryType>;

pub struct EntryCompare<'builder> {
    builder: &'builder AllProperties,
    path_value: Vec<u8>,
}

impl<'builder> EntryCompare<'builder> {
    pub fn new(builder: &'builder AllProperties, component: &OsStr) -> Self {
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

pub struct ReadEntry<'builder, Builder: BuilderTrait> {
    builder: &'builder Builder,
    current: jbk::EntryIdx,
    end: jbk::EntryIdx,
}

impl<'builder, Builder: BuilderTrait> ReadEntry<'builder, Builder> {
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

impl<'builder, Builder: BuilderTrait> Iterator for ReadEntry<'builder, Builder> {
    type Item = jbk::Result<Builder::Entry>;

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

    pub fn create_properties(&self, index: &jbk::reader::Index) -> jbk::Result<AllProperties> {
        AllProperties::new(
            index.get_store(self.get_entry_storage())?,
            self.get_value_storage(),
        )
    }

    pub fn root_index(&self) -> jbk::Result<jbk::reader::Index> {
        let directory = self.container.get_directory_pack();
        directory.get_index_from_name("arx_root")
    }
}
