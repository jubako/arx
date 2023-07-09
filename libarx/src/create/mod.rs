mod creator;
mod fs_adder;

pub use creator::Creator;
pub use fs_adder::FsAdder;

use std::ffi::{OsStr, OsString};

pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

pub enum EntryKind {
    Dir(Box<dyn Iterator<Item = jubako::Result<Box<dyn EntryTrait>>>>),
    File(jubako::Reader),
    Link(OsString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(self: Box<Self>) -> jubako::Result<EntryKind>;

    /// Under which name the entry will be stored
    fn name(&self) -> &OsStr;

    fn uid(&self) -> u64;
    fn gid(&self) -> u64;
    fn mode(&self) -> u64;
    fn mtime(&self) -> u64;
}

pub type Void = jubako::Result<()>;
