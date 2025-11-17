use super::mut_entry_store::{flat, DirEntry, Entry, Kind};
use crate::common::{EntryType, Property};
use jbk::creator::schema;
use jbk::Value;

use super::{EntryTrait, Void};

pub type ArxSchema = schema::Schema<Property, EntryType>;

type EntryStore = jbk::creator::EntryStore<Property, EntryType>;

pub struct EntryStoreCreator {
    schema: ArxSchema,
    path_store: jbk::creator::StoreHandle,
    root_entry: DirEntry,
}

#[derive(Debug)]
pub enum JbkKind {
    Dir {
        first_child: jbk::EntryIdx,
        nb_children: jbk::EntryCount,
    },
    File {
        content: jbk::ContentAddress,
        size: u64,
    },
    Link {
        target: bstr::BString,
    },
}

#[derive(Debug)]
pub struct JbkEntry {
    pub parent: Option<jbk::EntryIdx>,
    pub name: String,
    pub owner: u64,
    pub group: u64,
    pub rights: u64,
    pub mtime: u64,
    pub kind: JbkKind,
}

impl JbkEntry {
    pub(crate) fn new(e: Entry) -> (Self, Option<impl Iterator<Item = Entry>>) {
        let mut next_children = None;
        let s = Self {
            parent: e.parent,
            name: e.name,
            owner: e.owner,
            group: e.group,
            rights: e.rights,
            mtime: e.mtime,
            kind: match e.kind {
                Kind::File { content, size } => JbkKind::File { content, size },
                Kind::Link { target } => JbkKind::Link { target },
                Kind::Dir(d_entry) => {
                    let d = JbkKind::Dir {
                        nb_children: d_entry.nb_children(),
                        first_child: d_entry.first_child(),
                    };
                    next_children = Some(d_entry.children.into_values());
                    d
                }
            },
        };
        (s, next_children)
    }
}

impl jbk::creator::EntryTrait<Property, EntryType> for JbkEntry {
    fn variant_name(&self) -> Option<EntryType> {
        Some(match self.kind {
            JbkKind::File {
                content: _,
                size: _,
            } => EntryType::File,
            JbkKind::Link { target: _ } => EntryType::Link,
            JbkKind::Dir {
                nb_children: _,
                first_child: _,
            } => EntryType::Dir,
        })
    }

    fn value(&self, name: &Property) -> Value {
        match name {
            Property::Name => Value::Array(self.name.as_bytes().into()),
            Property::Parent => Value::Unsigned(self.parent.map_or(0, |p| p.into_u64() + 1)),
            Property::Owner => Value::Unsigned(self.owner),
            Property::Group => Value::Unsigned(self.group),
            Property::Rights => Value::Unsigned(self.rights),
            Property::Mtime => Value::Unsigned(self.mtime),
            Property::Content => {
                if let JbkKind::File { content, size: _ } = self.kind {
                    Value::Content(content)
                } else {
                    panic!("Should be a file")
                }
            }
            Property::Size => {
                if let JbkKind::File { content: _, size } = self.kind {
                    Value::Unsigned(size)
                } else {
                    panic!("Should be a file")
                }
            }
            Property::Target => {
                if let JbkKind::Link { target } = &self.kind {
                    Value::Array(target.to_vec().into())
                } else {
                    panic!("Should be a link")
                }
            }
            Property::FirstChild => {
                if let JbkKind::Dir {
                    first_child,
                    nb_children: _,
                } = self.kind
                {
                    Value::Unsigned(first_child.into_u64())
                } else {
                    panic!("Should be a dir")
                }
            }
            Property::NbChildren => {
                if let JbkKind::Dir {
                    first_child: _,
                    nb_children,
                } = self.kind
                {
                    Value::Unsigned(nb_children.into_u64())
                } else {
                    panic!("Should be a dir")
                }
            }
        }
    }

    fn value_count(&self) -> jbk::PropertyCount {
        match self.kind {
            JbkKind::Dir {
                first_child: _,
                nb_children: _,
            } => 6 + 2,
            JbkKind::File {
                content: _,
                size: _,
            } => 6 + 2,
            JbkKind::Link { target: _ } => 6 + 1,
        }
        .into()
    }
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

impl jbk::creator::EntryStoreCreatorTrait for EntryStoreCreator {
    fn finalize(self: Box<Self>, directory_pack: &mut jbk::creator::DirectoryPackCreator) {
        let entry_count = self.root_entry.nb_entry();
        let root_count = self.root_entry.nb_children();
        directory_pack.add_value_store(self.path_store);
        let flatten = flat(self.root_entry);
        let jbk_entry_store = EntryStore::new(self.schema, flatten);
        let entry_store_id = directory_pack.add_entry_store(jbk_entry_store);
        directory_pack.create_index(
            "arx_entries",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            entry_count,
            jbk::EntryIdx::from(0),
        );
        directory_pack.create_index(
            "arx_root",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            root_count,
            jbk::EntryIdx::from(0),
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
    use jbk::creator::EntryStoreCreatorTrait;
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
