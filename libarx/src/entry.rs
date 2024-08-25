use crate::common::{AllProperties, Builder};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;

#[derive(Clone)]
pub struct CommonPart {
    idx: jbk::EntryIdx,
    path: Vec<u8>,
    parent: Option<jbk::EntryIdx>,
    owner: u32,
    group: u32,
    rights: u8,
    mtime: u64,
}

pub trait CommonEntry {
    fn common(&self) -> &CommonPart;
    fn idx(&self) -> jbk::EntryIdx {
        self.common().idx
    }
    fn path(&self) -> &Vec<u8> {
        &self.common().path
    }
    fn parent(&self) -> Option<jbk::EntryIdx> {
        self.common().parent
    }
    fn owner(&self) -> u32 {
        self.common().owner
    }
    fn group(&self) -> u32 {
        self.common().group
    }
    fn rights(&self) -> u8 {
        self.common().rights
    }
    fn mtime(&self) -> u64 {
        self.common().mtime
    }
}

#[derive(Clone)]
pub struct FileEntry {
    common: CommonPart,
    content: jbk::ContentAddress,
    size: jbk::Size,
}

impl CommonEntry for FileEntry {
    fn common(&self) -> &CommonPart {
        &self.common
    }
}

impl FileEntry {
    pub fn content(&self) -> jbk::ContentAddress {
        self.content
    }
    pub fn size(&self) -> jbk::Size {
        self.size
    }
}

#[derive(Clone)]
pub struct Link {
    common: CommonPart,
    target: Vec<u8>,
}

impl CommonEntry for Link {
    fn common(&self) -> &CommonPart {
        &self.common
    }
}

impl Link {
    pub fn target(&self) -> &Vec<u8> {
        &self.target
    }
}

#[derive(Clone)]
pub struct Dir {
    common: CommonPart,
    range: jbk::EntryRange,
}

impl CommonEntry for Dir {
    fn common(&self) -> &CommonPart {
        &self.common
    }
}

impl Dir {
    pub fn range(&self) -> jbk::EntryRange {
        self.range
    }
}

mod private {
    use super::*;
    pub struct CommonBuilder {
        path_property: jbk::reader::builder::ArrayProperty,
        parent_prorperty: jbk::reader::builder::IntProperty,
        owner_property: jbk::reader::builder::IntProperty,
        group_property: jbk::reader::builder::IntProperty,
        rights_property: jbk::reader::builder::IntProperty,
        mtime_property: jbk::reader::builder::IntProperty,
    }

    impl CommonBuilder {
        fn new(properties: &AllProperties) -> Self {
            Self {
                path_property: properties.path_property.clone(),
                parent_prorperty: properties.parent_property.clone(),
                owner_property: properties.owner_property.clone(),
                group_property: properties.group_property.clone(),
                rights_property: properties.rigths_property.clone(),
                mtime_property: properties.mtime_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<CommonPart> {
            let path_prop = self.path_property.create(reader)?;
            let mut path = vec![];
            path_prop.resolve_to_vec(&mut path)?;
            let parent = self.parent_prorperty.create(reader)?;
            let parent = if parent == 0 {
                None
            } else {
                Some((parent as u32 - 1).into())
            };
            Ok(CommonPart {
                idx,
                path,
                parent,
                owner: self.owner_property.create(reader)? as u32,
                group: self.group_property.create(reader)? as u32,
                rights: self.rights_property.create(reader)? as u8,
                mtime: self.mtime_property.create(reader)?,
            })
        }
    }

    pub struct FileBuilder {
        common: CommonBuilder,
        content_address_property: jbk::reader::builder::ContentProperty,
        size_property: jbk::reader::builder::IntProperty,
    }

    impl Builder for FileBuilder {
        type Entry = FileEntry;

        fn new(properties: &AllProperties) -> Self {
            Self {
                common: CommonBuilder::new(properties),
                content_address_property: properties.file_content_address_property,
                size_property: properties.file_size_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
            Ok(FileEntry {
                common: self.common.create_entry(idx, reader)?,
                content: self.content_address_property.create(reader)?,
                size: self.size_property.create(reader)?.into(),
            })
        }
    }

    pub struct LinkBuilder {
        common: CommonBuilder,
        link_property: jbk::reader::builder::ArrayProperty,
    }

    impl Builder for LinkBuilder {
        type Entry = Link;

        fn new(properties: &AllProperties) -> Self {
            Self {
                common: CommonBuilder::new(properties),
                link_property: properties.link_target_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
            let common = self.common.create_entry(idx, reader)?;
            let target_prop = self.link_property.create(reader)?;
            let mut target = vec![];
            target_prop.resolve_to_vec(&mut target)?;
            Ok(Link { common, target })
        }
    }

    pub struct DirBuilder {
        common: CommonBuilder,
        first_child_property: jbk::reader::builder::IntProperty,
        nb_children_property: jbk::reader::builder::IntProperty,
    }

    impl Builder for DirBuilder {
        type Entry = Dir;

        fn new(properties: &AllProperties) -> Self {
            Self {
                common: CommonBuilder::new(properties),
                first_child_property: properties.dir_first_child_property.clone(),
                nb_children_property: properties.dir_nb_children_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
            let common = self.common.create_entry(idx, reader)?;
            let first_child: jbk::EntryIdx =
                (self.first_child_property.create(reader)? as u32).into();
            let nb_children: jbk::EntryCount =
                (self.nb_children_property.create(reader)? as u32).into();
            Ok(Dir {
                common,
                range: jbk::EntryRange::new_from_size(first_child, nb_children),
            })
        }
    }
} // private mode

pub type FullBuilder = (
    private::FileBuilder,
    private::LinkBuilder,
    private::DirBuilder,
);

pub type FullEntry = super::Entry<(FileEntry, Link, Dir)>;
