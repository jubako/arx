use jubako as jbk;
use std::rc::Rc;

pub struct Builder {
    pub(crate) store: Rc<jbk::reader::EntryStore>,
    pub(crate) path_property: jbk::reader::builder::ArrayProperty,
    pub(crate) parent_property: jbk::reader::builder::IntProperty,
    pub(crate) owner_property: jbk::reader::builder::IntProperty,
    pub(crate) group_property: jbk::reader::builder::IntProperty,
    pub(crate) rigths_property: jbk::reader::builder::IntProperty,
    pub(crate) mtime_property: jbk::reader::builder::IntProperty,
    pub(crate) variant_id_property: jbk::reader::builder::VariantIdProperty,
    pub(crate) file_content_address_property: jbk::reader::builder::ContentProperty,
    pub(crate) file_size_property: jbk::reader::builder::IntProperty,
    pub(crate) dir_first_child_property: jbk::reader::builder::IntProperty,
    pub(crate) dir_nb_children_property: jbk::reader::builder::IntProperty,
    pub(crate) link_target_property: jbk::reader::builder::ArrayProperty,
}

pub fn create_builder(
    store: Rc<jbk::reader::EntryStore>,
    value_storage: &jbk::reader::ValueStorage,
) -> jbk::Result<Builder> {
    let layout = store.layout();
    let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
    assert_eq!(variants.len(), 3);
    let path_property = (&layout.common[0], value_storage).try_into()?;
    let parent_property = (&layout.common[1], value_storage).try_into()?;
    let owner_property = (&layout.common[2], value_storage).try_into()?;
    let group_property = (&layout.common[3], value_storage).try_into()?;
    let rigths_property = (&layout.common[4], value_storage).try_into()?;
    let mtime_property = (&layout.common[5], value_storage).try_into()?;
    let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_offset);
    let file_content_address_property = (&variants[0][0]).try_into()?;
    let file_size_property = (&variants[0][1], value_storage).try_into()?;
    let dir_first_child_property = (&variants[1][0], value_storage).try_into()?;
    let dir_nb_children_property = (&variants[1][1], value_storage).try_into()?;
    let link_target_property = (&variants[2][0], value_storage).try_into()?;
    Ok(Builder {
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
