use super::common::*;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::rc::Rc;

pub use jbk::SubReader as Reader;

pub trait Operator<Context, FileEntry, LinkEntry, DirEntry> {
    fn on_start(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_stop(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_directory_enter(&self, context: &mut Context, entry: &DirEntry) -> jbk::Result<()>;
    fn on_directory_exit(&self, context: &mut Context, entry: &DirEntry) -> jbk::Result<()>;
    fn on_file(&self, context: &mut Context, entry: &FileEntry) -> jbk::Result<()>;
    fn on_link(&self, context: &mut Context, entry: &LinkEntry) -> jbk::Result<()>;
}

pub trait Builder {
    type Entry;

    fn new(properties: &AllProperties) -> Self;
    fn create_entry(&self, idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry>;
}

impl Builder for () {
    type Entry = ();
    fn new(_properties: &AllProperties) -> Self {}
    fn create_entry(&self, _idx: jbk::EntryIdx, _reader: &Reader) -> jbk::Result<Self::Entry> {
        Ok(())
    }
}

pub enum Entry<FileEntry, LinkEntry, DirEntry> {
    File(FileEntry),
    Link(LinkEntry),
    Dir(jbk::EntryRange, DirEntry),
}

pub(crate) struct WalkerBuilder<FileBuilder, LinkBuilder, DirBuilder> {
    store: Rc<jbk::reader::EntryStore>,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
    file_builder: FileBuilder,
    link_builder: LinkBuilder,
    dir_builder: DirBuilder,
}

impl<FileBuilder, LinkBuilder, DirBuilder> WalkerBuilder<FileBuilder, LinkBuilder, DirBuilder>
where
    FileBuilder: Builder,
    LinkBuilder: Builder,
    DirBuilder: Builder,
{
    pub fn new(properties: &AllProperties) -> Self {
        let file_builder = FileBuilder::new(properties);
        let link_builder = LinkBuilder::new(properties);
        let dir_builder = DirBuilder::new(properties);
        Self {
            store: Rc::clone(&properties.store),
            variant_id_property: properties.variant_id_property,
            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
            file_builder,
            link_builder,
            dir_builder,
        }
    }
}

impl<FileBuilder, LinkBuilder, DirBuilder> jbk::reader::builder::BuilderTrait
    for WalkerBuilder<FileBuilder, LinkBuilder, DirBuilder>
where
    FileBuilder: Builder,
    LinkBuilder: Builder,
    DirBuilder: Builder,
{
    type Entry = Entry<FileBuilder::Entry, LinkBuilder::Entry, DirBuilder::Entry>;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let file_type = self.variant_id_property.create(&reader)?.try_into()?;
        Ok(match file_type {
            EntryType::File => {
                let entry = self.file_builder.create_entry(idx, &reader)?;
                Entry::File(entry)
            }
            EntryType::Link => {
                let entry = self.link_builder.create_entry(idx, &reader)?;
                Entry::Link(entry)
            }
            EntryType::Dir => {
                let first_child: jbk::EntryIdx =
                    (self.first_child_property.create(&reader)? as u32).into();
                let nb_children: jbk::EntryCount =
                    (self.nb_children_property.create(&reader)? as u32).into();
                let range = jbk::EntryRange::new_from_size(first_child, nb_children);
                let entry = self.dir_builder.create_entry(idx, &reader)?;
                Entry::Dir(range, entry)
            }
        })
    }
}

pub struct Walker<'a, Context> {
    arx: &'a Arx,
    context: Context,
}

impl<'a, Context> Walker<'a, Context> {
    pub fn new(arx: &'a Arx, context: Context) -> Self {
        Self { arx, context }
    }

    pub fn run<FileBuilder, LinkBuilder, DirBuilder>(
        &mut self,
        index: jbk::reader::Index,
        op: &dyn Operator<Context, FileBuilder::Entry, LinkBuilder::Entry, DirBuilder::Entry>,
    ) -> jbk::Result<()>
    where
        FileBuilder: Builder,
        LinkBuilder: Builder,
        DirBuilder: Builder,
    {
        let properties = self.arx.create_properties(&index)?;
        let builder = WalkerBuilder::<FileBuilder, LinkBuilder, DirBuilder>::new(&properties);

        op.on_start(&mut self.context)?;
        self._run(&index, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    fn _run<R: Range, FileBuilder, LinkBuilder, DirBuilder>(
        &mut self,
        range: &R,
        builder: &WalkerBuilder<FileBuilder, LinkBuilder, DirBuilder>,
        op: &dyn Operator<Context, FileBuilder::Entry, LinkBuilder::Entry, DirBuilder::Entry>,
    ) -> jbk::Result<()>
    where
        FileBuilder: Builder,
        LinkBuilder: Builder,
        DirBuilder: Builder,
    {
        let read_entry = ReadEntry::new(range, builder);
        for entry in read_entry {
            match entry? {
                Entry::File(e) => op.on_file(&mut self.context, &e)?,
                Entry::Link(e) => op.on_link(&mut self.context, &e)?,
                Entry::Dir(range, e) => {
                    op.on_directory_enter(&mut self.context, &e)?;
                    self._run(&range, builder, op)?;
                    op.on_directory_exit(&mut self.context, &e)?;
                }
            }
        }
        Ok(())
    }
}
