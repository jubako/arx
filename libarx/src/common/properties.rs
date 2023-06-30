use jubako as jbk;

pub struct AllProperties {
    pub store: jbk::reader::EntryStore,
    pub path_property: jbk::reader::builder::ArrayProperty,
    pub parent_property: jbk::reader::builder::IntProperty,
    pub owner_property: jbk::reader::builder::IntProperty,
    pub group_property: jbk::reader::builder::IntProperty,
    pub rigths_property: jbk::reader::builder::IntProperty,
    pub mtime_property: jbk::reader::builder::IntProperty,
    pub variant_id_property: jbk::reader::builder::VariantIdProperty,
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
    ) -> jbk::Result<Self> {
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
