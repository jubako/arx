mod creator;
mod entry_store_creator;
mod fs_adder;

pub use creator::SimpleCreator;
pub use entry_store_creator::EntryStoreCreator;
pub use fs_adder::FsAdder;

#[derive(Clone)]
pub enum EntryKind {
    Dir,
    File(jbk::Size, jbk::ContentAddress),
    Link(bstr::BString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jbk::Result<Option<EntryKind>>;

    /// Under which name the entry will be stored
    fn path(&self) -> &crate::Path;

    fn uid(&self) -> u64;
    fn gid(&self) -> u64;
    fn mode(&self) -> u64;
    fn mtime(&self) -> u64;
}

pub type Void = jbk::Result<()>;
