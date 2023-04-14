use super::entry::*;
use super::entry_type::EntryType;
use super::{AllProperties, Reader};
use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::rc::Rc;

pub trait SimpleBuilder {
    type Entry;

    fn new(properties: &AllProperties) -> Self;
    fn create_entry(&self, idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry>;
}

impl SimpleBuilder for () {
    type Entry = ();
    fn new(_properties: &AllProperties) -> Self {}
    fn create_entry(&self, _idx: jbk::EntryIdx, _reader: &Reader) -> jbk::Result<Self::Entry> {
        Ok(())
    }
}

pub trait FullBuilder {
    type Entry: EntryDef;

    fn new(properties: &AllProperties) -> Self;
    fn create_file(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::File>;
    fn create_link(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Link>;
    fn create_dir(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Dir>;
}

impl<F, L, D> FullBuilder for (F, L, D)
where
    F: SimpleBuilder,
    L: SimpleBuilder,
    D: SimpleBuilder,
{
    type Entry = (F::Entry, L::Entry, D::Entry);

    fn new(properties: &AllProperties) -> Self {
        let file_builder = F::new(properties);
        let link_builder = L::new(properties);
        let dir_builder = D::new(properties);
        (file_builder, link_builder, dir_builder)
    }

    fn create_file(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::File> {
        self.0.create_entry(idx, reader)
    }

    fn create_link(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Link> {
        self.1.create_entry(idx, reader)
    }

    fn create_dir(
        &self,
        idx: jbk::EntryIdx,
        reader: &Reader,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Dir> {
        self.2.create_entry(idx, reader)
    }
}

pub(crate) struct Builder<B: FullBuilder> {
    store: Rc<jbk::reader::EntryStore>,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
    builder: B,
}

impl<B> Builder<B>
where
    B: FullBuilder,
{
    pub fn new(properties: &AllProperties) -> Self {
        let builder = B::new(properties);
        Self {
            store: Rc::clone(&properties.store),
            variant_id_property: properties.variant_id_property,
            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
            builder,
        }
    }
}

impl<B> jbk::reader::builder::BuilderTrait for Builder<B>
where
    B: FullBuilder,
{
    type Entry = Entry<B::Entry>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let file_type = self.variant_id_property.create(&reader)?.try_into()?;
        Ok(match file_type {
            EntryType::File => {
                let entry = self.builder.create_file(idx, &reader)?;
                Entry::File(entry)
            }
            EntryType::Link => {
                let entry = self.builder.create_link(idx, &reader)?;
                Entry::Link(entry)
            }
            EntryType::Dir => {
                let first_child: jbk::EntryIdx =
                    (self.first_child_property.create(&reader)? as u32).into();
                let nb_children: jbk::EntryCount =
                    (self.nb_children_property.create(&reader)? as u32).into();
                let range = jbk::EntryRange::new_from_size(first_child, nb_children);
                let entry = self.builder.create_dir(idx, &reader)?;
                Entry::Dir(range, entry)
            }
        })
    }
}
