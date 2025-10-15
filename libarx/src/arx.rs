use super::common::{AllProperties, Comparator, Entry, FullBuilderTrait, RealBuilder};
use super::error::{ArxError, ArxFormatError, PathNotFound, QueryError};
use jbk::reader::builder::BuilderTrait;
use jbk::{reader::Range, EntryIdx};
use std::path::Path;

pub struct Arx {
    pub container: jbk::reader::Container,
    pub root_index: jbk::reader::Index,
    pub properties: AllProperties,
}

impl std::ops::Deref for Arx {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

fn create_properties(
    container: &jbk::reader::Container,
    index: &jbk::reader::Index,
) -> Result<AllProperties, ArxError> {
    Ok(AllProperties::new(
        index.get_store(container.get_entry_storage())?,
        container.get_value_storage(),
    )?)
}

impl Arx {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self, ArxError> {
        let container = jbk::reader::Container::new(&file)?;
        let root_index = container
            .get_directory_pack()
            .get_index_from_name("arx_root")?
            .ok_or(ArxFormatError("No arx_root index"))?;
        let properties = create_properties(&container, &root_index)?;
        Ok(Self {
            container,
            root_index,
            properties,
        })
    }

    pub fn create_properties(&self, index: &jbk::reader::Index) -> Result<AllProperties, ArxError> {
        create_properties(&self.container, index)
    }

    pub fn get_entry<B>(&self, path: &crate::Path) -> Result<Entry<B::Entry>, QueryError>
    where
        B: FullBuilderTrait,
    {
        self.get_entry_in_range::<B, _>(path, &self.root_index)
    }

    pub fn get_entry_in_range<B, R>(
        &self,
        path: &crate::Path,
        range: &R,
    ) -> Result<Entry<B::Entry>, QueryError>
    where
        B: FullBuilderTrait,
        R: jbk::reader::Range,
    {
        let comparator = Comparator::new(&self.properties);
        let builder = RealBuilder::<B>::new(&self.properties);
        let mut current_range = jbk::EntryRange::from_range(range);
        let mut components = path.iter().peekable();
        while let Some(component) = components.next() {
            // Search for the current component.
            // All children of a parent are stored concatened.
            // So if parent_id is different than current_parent,
            // we know we are out of the directory
            let comparator = comparator.compare_with(component.as_bytes());
            let found = current_range.find(&comparator)?;
            match found {
                None => return Err(PathNotFound(path.into()).into()),
                Some(idx) => {
                    let entry = current_range
                        .get_entry(&builder, idx)?
                        .expect("idx is valid");
                    if components.peek().is_none() {
                        // We have the last component
                        return Ok(entry);
                    } else if let Entry::Dir(range, _) = entry {
                        current_range = range;
                    } else {
                        return Err(PathNotFound(path.into()).into());
                    }
                }
            }
        }
        unreachable!();
    }

    pub fn get_entry_at_idx<B>(&self, idx: EntryIdx) -> Result<Option<Entry<B::Entry>>, ArxError>
    where
        B: FullBuilderTrait,
    {
        let builder = RealBuilder::<B>::new(&self.properties);
        Ok(builder.create_entry(idx)?)
    }
}
