mod creator;
mod entry_store_creator;
mod fs_adder;

pub use creator::FsCreator;
pub use entry_store_creator::EntryStoreCreator;
pub use fs_adder::{Adder, FsAdder};
use std::path::Path;

use std::ffi::OsString;

#[derive(Clone, Copy)]
pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

#[derive(Clone)]
pub enum EntryKind {
    Dir,
    File(jubako::Size, jubako::ContentAddress),
    Link(OsString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jubako::Result<Option<EntryKind>>;

    /// Under which name the entry will be stored
    fn path(&self) -> &Path;

    fn uid(&self) -> u64;
    fn gid(&self) -> u64;
    fn mode(&self) -> u64;
    fn mtime(&self) -> u64;
}

pub type Void = jubako::Result<()>;
