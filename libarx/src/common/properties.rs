use super::EntryType;
use crate::{ArxFormatError, BaseError};

jbk::properties! {
    Property {
        Name:"array" => "name",
        Parent:"int" => "parent",
        Owner:"int" => "owner",
        Group:"int" => "group",
        Rights:"int" => "rights",
        Mtime:"int" => "mtime",
        Content:"content" => "content",
        Size:"int" => "size",
        FirstChild:"int" => "first_child",
        NbChildren:"int" => "nb_children",
        Target:"array" => "target",
    }
}

pub struct AllProperties {
    pub store: jbk::reader::EntryStore,
    pub path_property: jbk::reader::builder::ArrayProperty,
    pub parent_property: jbk::reader::builder::IntProperty,
    pub owner_property: jbk::reader::builder::IntProperty,
    pub group_property: jbk::reader::builder::IntProperty,
    pub rigths_property: jbk::reader::builder::IntProperty,
    pub mtime_property: jbk::reader::builder::IntProperty,
    pub variant_id_property: jbk::reader::builder::VariantIdBuilder<EntryType>,
    pub file_content_address_property: jbk::reader::builder::ContentProperty,
    pub file_size_property: jbk::reader::builder::IntProperty,
    pub dir_first_child_property: jbk::reader::builder::IntProperty,
    pub dir_nb_children_property: jbk::reader::builder::IntProperty,
    pub link_target_property: jbk::reader::builder::ArrayProperty,
}

impl AllProperties {
    pub fn new(
        store: jbk::reader::EntryStore,
        value_storage: &jbk::reader::ValueStorage,
    ) -> Result<Self, BaseError> {
        let layout = store.layout();
        if layout.variant_len() != 3 {
            return Err(ArxFormatError("Layout must contain 3 variants").into());
        }
        let path_property = jbk::layout_builder!(
            layout[common][Property::Name],
            value_storage,
            ArxFormatError
        );
        let parent_property = jbk::layout_builder!(
            layout[common][Property::Parent],
            value_storage,
            ArxFormatError
        );
        let owner_property = jbk::layout_builder!(
            layout[common][Property::Owner],
            value_storage,
            ArxFormatError
        );
        let group_property = jbk::layout_builder!(
            layout[common][Property::Group],
            value_storage,
            ArxFormatError
        );
        let rigths_property = jbk::layout_builder!(
            layout[common][Property::Rights],
            value_storage,
            ArxFormatError
        );
        let mtime_property = jbk::layout_builder!(
            layout[common][Property::Mtime],
            value_storage,
            ArxFormatError
        );
        let variant_id_property = layout.variant_id_builder().expect("We have variants");
        let file_content_address_property = jbk::layout_builder!(
            layout[EntryType::File][Property::Content],
            value_storage,
            ArxFormatError
        );
        let file_size_property = jbk::layout_builder!(
            layout[EntryType::File][Property::Size],
            value_storage,
            ArxFormatError
        );
        let dir_first_child_property = jbk::layout_builder!(
            layout[EntryType::Dir][Property::FirstChild],
            value_storage,
            ArxFormatError
        );
        let dir_nb_children_property = jbk::layout_builder!(
            layout[EntryType::Dir][Property::NbChildren],
            value_storage,
            ArxFormatError
        );
        let link_target_property = jbk::layout_builder!(
            layout[EntryType::Link][Property::Target],
            value_storage,
            ArxFormatError
        );
        Ok(Self {
            store,
            path_property,
            parent_property,
            owner_property,
            group_property,
            rigths_property,
            mtime_property,
            variant_id_property,
            file_content_address_property,
            file_size_property,
            dir_first_child_property,
            dir_nb_children_property,
            link_target_property,
        })
    }
}
