use crate::common::{EntryType, Property};
use jbk::creator::schema;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::rc::Rc;

use super::{EntryKind, EntryTrait, Void};

type EntryStore = jbk::creator::EntryStore<
    Property,
    EntryType,
    Box<jbk::creator::BasicEntry<Property, EntryType>>,
>;

type DirCache = HashMap<OsString, DirEntry>;
type EntryIdx = jbk::Bound<jbk::EntryIdx>;

/// A DirEntry structure to keep track of added direcotry in the archive.
/// This is needed as we may adde file without recursion, and so we need
/// to find the parent of "foo/bar/baz.txt" ("foo/bar") when we add it.
struct DirEntry {
    idx: Option<EntryIdx>,
    dir_children: Rc<RefCell<DirCache>>,
    file_children: Rc<Vec<EntryIdx>>,
}

impl DirEntry {
    fn new_root() -> Self {
        Self {
            idx: None,
            dir_children: Default::default(),
            file_children: Default::default(),
        }
    }
    fn new(idx: EntryIdx) -> Self {
        Self {
            idx: Some(idx),
            dir_children: Default::default(),
            file_children: Default::default(),
        }
    }

    fn first_entry_generator(&self) -> Box<dyn Fn() -> u64> {
        let dir_children = Rc::clone(&self.dir_children);
        let file_children = Rc::clone(&self.file_children);
        Box::new(move || {
            if dir_children.borrow().is_empty() && file_children.is_empty() {
                0
            } else {
                std::cmp::min(
                    file_children
                        .iter()
                        .map(|i| i.get().into_u64())
                        .min()
                        .unwrap_or(u64::MAX),
                    dir_children
                        .borrow()
                        .values()
                        // Unwrap is safe because children are not root, and idx is Some
                        .map(|i| i.idx.as_ref().unwrap().get().into_u64())
                        .min()
                        .unwrap_or(u64::MAX),
                )
            }
        })
    }

    fn entry_count_generator(&self) -> Box<dyn Fn() -> u64> {
        let dir_children = Rc::clone(&self.dir_children);
        let file_children = Rc::clone(&self.file_children);
        Box::new(move || (dir_children.borrow().len() + file_children.len()) as u64)
    }

    fn as_parent_idx_generator(&self) -> Box<dyn Fn() -> u64> {
        match &self.idx {
            Some(idx) => {
                let idx = idx.clone();
                Box::new(move || idx.get().into_u64() + 1)
            }
            None => Box::new(|| 0),
        }
    }

    fn add<'a, E, C>(&mut self, entry: &E, mut components: C, entry_store: &mut EntryStore) -> Void
    where
        E: EntryTrait + ?Sized,
        C: Iterator<Item = std::path::Component<'a>>,
    {
        match components.next() {
            None => self.add_entry(entry, entry_store),
            Some(component) => self
                .dir_children
                .borrow_mut()
                .get_mut(component.as_os_str())
                .unwrap()
                .add(entry, components, entry_store),
        }
    }

    fn add_entry<E>(&mut self, entry: &E, entry_store: &mut EntryStore) -> Void
    where
        E: EntryTrait + ?Sized,
    {
        let entry_kind = match entry.kind()? {
            Some(k) => k,
            None => {
                return Ok(());
            }
        };
        let entry_name = entry
            .path()
            .file_name()
            .unwrap_or_else(|| panic!("{:?} has no file name", entry.path()))
            .to_os_string();
        let mut values = HashMap::from([
            (
                Property::Name,
                jbk::Value::Array(entry_name.clone().into_vec()),
            ),
            (
                Property::Parent,
                jbk::Value::Unsigned(self.as_parent_idx_generator().into()),
            ),
            (Property::Owner, jbk::Value::Unsigned(entry.uid().into())),
            (Property::Group, jbk::Value::Unsigned(entry.gid().into())),
            (Property::Rights, jbk::Value::Unsigned(entry.mode().into())),
            (Property::Mtime, jbk::Value::Unsigned(entry.mtime().into())),
        ]);

        match entry_kind {
            EntryKind::Dir => {
                if self.dir_children.borrow().contains_key(&entry_name) {
                    return Ok(());
                }
                let entry_idx = jbk::Vow::new(jbk::EntryIdx::from(0));
                let dir_entry = DirEntry::new(entry_idx.bind());

                {
                    values.insert(
                        Property::FirstChild,
                        jbk::Value::Unsigned(dir_entry.first_entry_generator().into()),
                    );
                    values.insert(
                        Property::NbChildren,
                        jbk::Value::Unsigned(dir_entry.entry_count_generator().into()),
                    );
                    let entry = Box::new(jbk::creator::BasicEntry::new_from_schema_idx(
                        &entry_store.schema,
                        entry_idx,
                        Some(EntryType::Dir),
                        values,
                    ));
                    entry_store.add_entry(entry);
                }

                self.dir_children
                    .borrow_mut()
                    .entry(entry_name)
                    .or_insert(dir_entry);
                Ok(())
            }
            EntryKind::File(size, content_address) => {
                values.insert(Property::Content, jbk::Value::Content(content_address));
                values.insert(Property::Size, jbk::Value::Unsigned(size.into_u64().into()));
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::File),
                    values,
                ));
                let current_idx = entry_store.add_entry(entry);
                /* SAFETY: We already have Rc on `self.file_children` but it is only used
                  in a second step to get entry_count and min entry_idx.
                  So while we borrow `self.file_children` we never read it otherwise.
                */
                unsafe { Rc::get_mut_unchecked(&mut self.file_children) }.push(current_idx);
                Ok(())
            }
            EntryKind::Link(target) => {
                values.insert(Property::Target, jbk::Value::Array(target.into_vec()));
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &entry_store.schema,
                    Some(EntryType::Link),
                    values,
                ));
                let current_idx = entry_store.add_entry(entry);
                /* SAFETY: We already have Rc on `self.file_children` but it is only used
                  in a second step to get entry_count and min entry_idx.
                  So while we borrow `self.file_children` we never read it otherwise.
                */
                unsafe { Rc::get_mut_unchecked(&mut self.file_children) }.push(current_idx);
                Ok(())
            }
        }
    }
}

pub struct EntryStoreCreator {
    entry_store: Box<EntryStore>,
    path_store: jbk::creator::ValueStore,
    root_entry: DirEntry,
}

impl EntryStoreCreator {
    pub fn new() -> Self {
        let path_store = jbk::creator::ValueStore::new_plain();

        let entry_def = schema::Schema::new(
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

        let entry_store = Box::new(EntryStore::new(entry_def));

        let root_entry = DirEntry::new_root();

        Self {
            entry_store,
            path_store,
            root_entry,
        }
    }

    pub fn finalize(self, directory_pack: &mut jbk::creator::DirectoryPackCreator) {
        let root_count = self.entry_count();
        let entry_count = self.entry_store.len();
        directory_pack.add_value_store(self.path_store);
        let entry_store_id = directory_pack.add_entry_store(self.entry_store);
        directory_pack.create_index(
            "arx_entries",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(entry_count as u32),
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

    pub fn entry_count(&self) -> jbk::EntryCount {
        jbk::EntryCount::from(self.root_entry.entry_count_generator()() as u32)
    }

    pub fn add_entry<E>(&mut self, entry: &E) -> Void
    where
        E: EntryTrait,
    {
        let path = entry.path().to_path_buf();
        match path.parent() {
            None => self
                .root_entry
                .add(entry, std::iter::empty(), &mut self.entry_store),
            Some(parent) => self
                .root_entry
                .add(entry, parent.components(), &mut self.entry_store),
        }
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
    use std::path::{Path, PathBuf};

    #[test]
    fn test_empty() -> jbk::Result<()> {
        let mut arx_file = tempfile::NamedTempFile::new_in(&std::env::temp_dir())?;
        let mut creator =
            jbk::creator::DirectoryPackCreator::new(jbk::PackId::from(0), 0, Default::default());

        let entry_store_creator = EntryStoreCreator::new();
        entry_store_creator.finalize(&mut creator);
        creator.finalize(&mut arx_file)?;
        assert!(arx_file.path().is_file());

        let directory_pack = jbk::reader::DirectoryPack::new(
            jbk::creator::FileSource::open(arx_file.path())?.into(),
        )?;
        let index = directory_pack.get_index_from_name("arx_entries")?;
        assert!(index.is_empty());
        Ok(())
    }

    struct SimpleEntry(PathBuf);

    impl EntryTrait for SimpleEntry {
        fn path(&self) -> &Path {
            &self.0
        }

        fn kind(&self) -> jbk::Result<Option<EntryKind>> {
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
    fn test_one_content() -> jbk::Result<()> {
        let mut arx_file = tempfile::NamedTempFile::new_in(&std::env::temp_dir())?;

        let mut creator =
            jbk::creator::DirectoryPackCreator::new(jbk::PackId::from(0), 0, Default::default());

        let mut entry_store_creator = EntryStoreCreator::new();
        let entry = SimpleEntry("foo.txt".into());
        entry_store_creator.add_entry(&entry)?;
        entry_store_creator.finalize(&mut creator);
        creator.finalize(&mut arx_file)?;
        assert!(arx_file.path().is_file());

        let directory_pack = jbk::reader::DirectoryPack::new(
            jbk::creator::FileSource::open(arx_file.path())?.into(),
        )?;
        let index = directory_pack.get_index_from_name("arx_entries")?;
        assert!(!index.is_empty());
        Ok(())
    }
}
