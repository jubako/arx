use std::fmt::Display;

jbk::variants! {
    EntryType {
        File => "file",
        Dir => "dir",
        Link => "link"
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
