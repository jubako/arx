use jubako as jbk;

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

impl From<EntryType> for String {
    fn from(t: EntryType) -> Self {
        match &t {
            EntryType::File => String::from("file"),
            EntryType::Dir => String::from("dir"),
            EntryType::Link => String::from("link"),
        }
    }
}

impl From<EntryType> for fuser::FileType {
    fn from(t: EntryType) -> Self {
        match t {
            EntryType::File => fuser::FileType::RegularFile,
            EntryType::Dir => fuser::FileType::Directory,
            EntryType::Link => fuser::FileType::Symlink,
        }
    }
}
