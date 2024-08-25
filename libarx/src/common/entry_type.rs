#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum EntryType {
    File = 0,
    Dir = 1,
    Link = 2,
}

impl TryFrom<jbk::VariantIdx> for EntryType {
    type Error = String;
    fn try_from(id: jbk::VariantIdx) -> Result<Self, Self::Error> {
        match id.into_u8() {
            0 => Ok(Self::File),
            1 => Ok(Self::Dir),
            2 => Ok(Self::Link),
            _ => Err("Invalid variant id".into()),
        }
    }
}

impl ToString for EntryType {
    fn to_string(&self) -> String {
        match self {
            EntryType::File => String::from("file"),
            EntryType::Dir => String::from("dir"),
            EntryType::Link => String::from("link"),
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
