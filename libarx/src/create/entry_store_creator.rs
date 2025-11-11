use super::mut_entry_store::{flat, DirEntry, Entry, Kind};
use crate::common::{EntryType, Property};
use jbk::creator::schema;
use std::collections::HashMap;

use super::{EntryTrait, Void};

pub type JbkEntry = jbk::creator::BasicEntry<Property, EntryType>;
pub type ArxSchema = schema::Schema<Property, EntryType>;

type EntryStore =
    jbk::creator::EntryStore<Property, EntryType, jbk::creator::BasicEntry<Property, EntryType>>;

pub fn to_basic_entry(entry: &Entry, schema: &ArxSchema) -> JbkEntry {
    let mut values = HashMap::from([
        (
            Property::Name,
            jbk::Value::Array(entry.name.as_bytes().into()),
        ),
        (
            Property::Parent,
            jbk::Value::Unsigned(entry.parent.map_or(0, |p| p.into_u64() + 1)),
        ),
        (Property::Owner, jbk::Value::Unsigned(entry.owner)),
        (Property::Group, jbk::Value::Unsigned(entry.group)),
        (Property::Rights, jbk::Value::Unsigned(entry.rights)),
        (Property::Mtime, jbk::Value::Unsigned(entry.mtime)),
    ]);

    let entry_type = match &entry.kind {
        Kind::Dir(d) => {
            values.insert(
                Property::FirstChild,
                jbk::Value::Unsigned(d.first_child().into_u64()),
            );
            values.insert(
                Property::NbChildren,
                jbk::Value::Unsigned(d.nb_children().into_u64()),
            );
            EntryType::Dir
        }
        Kind::File { content, size } => {
            values.insert(Property::Content, jbk::Value::Content(*content));
            values.insert(Property::Size, jbk::Value::Unsigned(*size));
            EntryType::File
        }
        Kind::Link { target } => {
            values.insert(Property::Target, jbk::Value::Array(target.to_vec().into()));
            EntryType::Link
        }
    };
    jbk::creator::BasicEntry::new_from_schema(schema, Some(entry_type), values)
}

pub struct EntryStoreCreator {
    schema: schema::Schema<Property, EntryType>,
    path_store: jbk::creator::StoreHandle,
    root_entry: DirEntry,
}

impl EntryStoreCreator {
    pub fn new() -> Self {
        let path_store = jbk::creator::ValueStore::new_plain(None);

        let schema = schema::Schema::new(
            // Common part
            schema::CommonProperties::new(vec![
                schema::Property::new_array(1, path_store.clone(), Property::Name), // the path
                schema::Property::new_uint(Property::Parent), // index of the parent entry
                schema::Property::new_uint(Property::Owner),  // owner
                schema::Property::new_uint(Property::Group),  // group
                schema::Property::new_uint(Property::Rights), // rights
                schema::Property::new_uint(Property::Mtime),  // modification time
            ]),
            vec![
                // File
                (
                    EntryType::File,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_content_address(Property::Content),
                        schema::Property::new_uint(Property::Size), // Size
                    ]),
                ),
                // Directory
                (
                    EntryType::Dir,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_uint(Property::FirstChild), // index of the first entry
                        schema::Property::new_uint(Property::NbChildren), // nb entries in the directory
                    ]),
                ),
                // Link
                (
                    EntryType::Link,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_array(1, path_store.clone(), Property::Target), // Id of the linked entry
                    ]),
                ),
            ],
            Some(vec![Property::Parent, Property::Name]),
        );
        let root_entry = DirEntry::new();
        Self {
            schema,
            path_store,
            root_entry,
        }
    }

    pub fn add_entry<E>(&mut self, entry: &E) -> Void
    where
        E: EntryTrait,
    {
        let path = entry.path();
        match path.parent() {
            None => self.root_entry.add(entry, std::iter::empty()),
            Some(parent) => self.root_entry.add(entry, parent.components()),
        }
    }
}

impl jbk::creator::EntryStoreTrait for EntryStoreCreator {
    fn finalize(self: Box<Self>, directory_pack: &mut jbk::creator::DirectoryPackCreator) {
        let entry_count = self.root_entry.nb_entry();
        let root_count = self.root_entry.nb_children();
        directory_pack.add_value_store(self.path_store);
        let flatten = flat(self.root_entry, &self.schema);
        let mut jbk_entry_store = EntryStore::new(self.schema, Some(flatten.len()));
        for entry in flatten {
            jbk_entry_store.add_entry(entry);
        }
        let entry_store_id = directory_pack.add_entry_store(Box::new(jbk_entry_store));
        directory_pack.create_index(
            "arx_entries",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            entry_count,
            jbk::EntryIdx::from(0).into(),
        );
        directory_pack.create_index(
            "arx_root",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            root_count,
            jbk::EntryIdx::from(0).into(),
        );
    }
}

impl Default for EntryStoreCreator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use jbk::creator::EntryStoreTrait;
    use rustest::{test, *};

    #[test]
    fn test_empty() -> Result {
        let arx_file = tempfile::NamedTempFile::new_in(std::env::temp_dir())?;
        let (mut arx_file, arx_name) = arx_file.into_parts();
        let mut creator = jbk::creator::DirectoryPackCreator::new(
            jbk::PackId::from(0),
            crate::VENDOR_ID,
            Default::default(),
        );

        let entry_store_creator = Box::new(EntryStoreCreator::new());
        entry_store_creator.finalize(&mut creator);
        creator.finalize()?.write(&mut arx_file)?;
        assert!(arx_name.is_file());

        let directory_pack =
            jbk::reader::DirectoryPack::new(jbk::creator::FileSource::open(arx_name)?.into())?;
        let index = directory_pack
            .get_index_from_name("arx_entries")?
            .expect("arx_entries should exists.");
        assert!(index.is_empty());
        Ok(())
    }

    struct SimpleEntry(crate::PathBuf);

    impl EntryTrait for SimpleEntry {
        fn path(&self) -> &crate::Path {
            &self.0
        }

        fn kind(&self) -> std::result::Result<Option<EntryKind>, crate::error::CreatorError> {
            Ok(Some(EntryKind::File(
                jbk::Size::new(10),
                jbk::ContentAddress::new(1.into(), 0.into()),
            )))
        }

        fn uid(&self) -> u64 {
            1000
        }

        fn gid(&self) -> u64 {
            1000
        }

        fn mode(&self) -> u64 {
            0o777
        }

        fn mtime(&self) -> u64 {
            0
        }
    }

    #[test]
    fn test_one_content() -> Result {
        let arx_file = tempfile::NamedTempFile::new_in(std::env::temp_dir())?;
        let (mut arx_file, arx_name) = arx_file.into_parts();

        let mut creator = jbk::creator::DirectoryPackCreator::new(
            jbk::PackId::from(0),
            crate::VENDOR_ID,
            Default::default(),
        );

        let mut entry_store_creator = Box::new(EntryStoreCreator::new());
        let entry = SimpleEntry("foo.txt".into());
        entry_store_creator.add_entry(&entry)?;
        entry_store_creator.finalize(&mut creator);
        creator.finalize()?.write(&mut arx_file)?;
        assert!(arx_name.is_file());

        let directory_pack =
            jbk::reader::DirectoryPack::new(jbk::creator::FileSource::open(arx_name)?.into())?;
        let index = directory_pack
            .get_index_from_name("arx_entries")?
            .expect("arx_entries should exists.");
        assert!(!index.is_empty());
        Ok(())
    }
}
