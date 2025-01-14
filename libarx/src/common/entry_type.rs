use std::fmt::Display;

use crate::ArxFormatError;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum EntryType {
    File = 0,
    Dir = 1,
    Link = 2,
}

impl TryFrom<jbk::VariantIdx> for EntryType {
    type Error = ArxFormatError;
    fn try_from(id: jbk::VariantIdx) -> Result<Self, Self::Error> {
        match id.into_u8() {
            0 => Ok(Self::File),
            1 => Ok(Self::Dir),
            2 => Ok(Self::Link),
            _ => Err(ArxFormatError("Invalid variant id")),
        }
    }
}

impl Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::File => write!(f, "file"),
            EntryType::Dir => write!(f, "dir"),
            EntryType::Link => write!(f, "link"),
        }
    }
}

impl jbk::creator::VariantName for EntryType {}

#[cfg(all(not(windows), feature = "fuse"))]
impl From<EntryType> for fuser::FileType {
    fn from(t: EntryType) -> Self {
        match t {
            EntryType::File => fuser::FileType::RegularFile,
            EntryType::Dir => fuser::FileType::Directory,
            EntryType::Link => fuser::FileType::Symlink,
        }
    }
}
