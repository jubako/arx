#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum Property {
    Name,
    Parent,
    Owner,
    Group,
    Rights,
    Mtime,
    Content,
    Size,
    FirstChild,
    NbChildren,
    Target,
}

impl ToString for Property {
    fn to_string(&self) -> String {
        use Property::*;
        String::from(match self {
            Name => "name",
            Parent => "parent",
            Owner => "owner",
            Group => "group",
            Rights => "rights",
            Mtime => "mtime",
            Content => "content",
            Size => "size",
            FirstChild => "first_child",
            NbChildren => "nb_children",
            Target => "target",
        })
    }
}

impl jbk::creator::PropertyName for Property {}

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
        let jbk::reader::layout::VariantPart {
            variant_id_offset,
            variants,
            names,
        } = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 3);
        let path_property = (&layout.common["name"], value_storage).try_into()?;
        let parent_property = (&layout.common["parent"], value_storage).try_into()?;
        let owner_property = (&layout.common["owner"], value_storage).try_into()?;
        let group_property = (&layout.common["group"], value_storage).try_into()?;
        let rigths_property = (&layout.common["rights"], value_storage).try_into()?;
        let mtime_property = (&layout.common["mtime"], value_storage).try_into()?;
        let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_id_offset);
        let file_content_address_property =
            (&variants[names["file"] as usize]["content"]).try_into()?;
        let file_size_property =
            (&variants[names["file"] as usize]["size"], value_storage).try_into()?;
        let dir_first_child_property = (
            &variants[names["dir"] as usize]["first_child"],
            value_storage,
        )
            .try_into()?;
        let dir_nb_children_property = (
            &variants[names["dir"] as usize]["nb_children"],
            value_storage,
        )
            .try_into()?;
        let link_target_property =
            (&variants[names["link"] as usize]["target"], value_storage).try_into()?;
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
