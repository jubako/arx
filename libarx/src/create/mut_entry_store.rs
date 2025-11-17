use super::entry_store_creator::JbkEntry;
use crate::IncoherentStructure;
use std::collections::{BTreeMap, VecDeque};

use super::{EntryKind, EntryTrait, Void};

pub enum Kind {
    Dir(DirEntry),
    File {
        content: jbk::ContentAddress,
        size: u64,
    },
    Link {
        target: bstr::BString,
    },
}

pub struct Entry {
    pub idx: Option<jbk::EntryIdx>,
    pub parent: Option<jbk::EntryIdx>,
    pub name: String,
    pub owner: u64,
    pub group: u64,
    pub rights: u64,
    pub mtime: u64,
    pub kind: Kind,
}

impl Entry {
    fn new_dir(name: String, owner: u64, group: u64, rights: u64, mtime: u64) -> Self {
        Self {
            idx: None,
            parent: None,
            name,
            owner,
            group,
            rights,
            mtime,
            kind: Kind::Dir(DirEntry::new()),
        }
    }
    fn new_file(
        name: String,
        owner: u64,
        group: u64,
        rights: u64,
        mtime: u64,
        content: jbk::ContentAddress,
        size: u64,
    ) -> Self {
        Self {
            idx: None,
            parent: None,
            name,
            owner,
            group,
            rights,
            mtime,
            kind: Kind::File { content, size },
        }
    }
    fn new_link(
        name: String,
        owner: u64,
        group: u64,
        rights: u64,
        mtime: u64,
        target: bstr::BString,
    ) -> Self {
        Self {
            idx: None,
            parent: None,
            name,
            owner,
            group,
            rights,
            mtime,
            kind: Kind::Link { target },
        }
    }

    fn nb_children(&self) -> jbk::EntryCount {
        match &self.kind {
            Kind::Dir(dir) => dir.nb_entry(),
            _ => 0.into(),
        }
    }

    fn set_idx(&mut self, idx: jbk::EntryIdx, parent: Option<jbk::EntryIdx>) {
        self.idx = Some(idx);
        self.parent = parent;
    }
}

type DirCache = BTreeMap<String, Entry>;

/// A DirEntry structure to keep track of added direcotry in the archive.
/// This is needed as we may adde file without recursion, and so we need
/// to find the parent of "foo/bar/baz.txt" ("foo/bar") when we add it.
pub struct DirEntry {
    pub children: DirCache,
}

impl DirEntry {
    pub fn new() -> Self {
        Self {
            children: Default::default(),
        }
    }

    pub fn first_child(&self) -> jbk::EntryIdx {
        self.children
            .first_key_value()
            .map_or(jbk::EntryIdx::from(0), |(_, v)| v.idx.unwrap())
    }

    pub fn nb_children(&self) -> jbk::EntryCount {
        jbk::EntryCount::from(self.children.len() as u32)
    }

    pub fn nb_entry(&self) -> jbk::EntryCount {
        let mut nb_entry = self.nb_children();
        for child in self.children.values() {
            nb_entry += child.nb_children().into_u32();
        }
        nb_entry
    }

    pub fn add<'a, E, C>(&mut self, entry: &E, mut components: C) -> Void
    where
        E: EntryTrait + ?Sized,
        C: Iterator<Item = relative_path::Component<'a>>,
    {
        match components.next() {
            None => self.add_entry(entry),
            Some(component) => {
                self.ensure_dir(component.as_str())?;
                let write_children = &mut self.children;
                match &mut write_children.get_mut(component.as_str()).unwrap().kind {
                    Kind::Dir(e) => e.add(entry, components),
                    Kind::File{content: _, size:_} => Err(IncoherentStructure(format!(
                        "Adding {}, cannot add a entry to something which is not a directory (file)",
                        entry.path()
                    ))
                    .into()),
                    Kind::Link{target:_} => Err(IncoherentStructure(format!(
                        "Adding {}, cannot add a entry to something which is not a directory (link)",
                        entry.path()
                    ))
                    .into()),
                }
            }
        }
    }

    fn ensure_dir(&mut self, dir_name: &str) -> Void {
        self.children
            .entry(dir_name.into())
            .or_insert_with(|| Entry::new_dir(dir_name.into(), 1000, 1000, 0o755, 0));

        Ok(())
    }

    fn add_entry<E>(&mut self, entry: &E) -> Void
    where
        E: EntryTrait + ?Sized,
    {
        let entry_kind = match entry.kind()? {
            Some(k) => k,
            None => {
                return Ok(());
            }
        };
        let entry_name = entry
            .path()
            .file_name()
            .unwrap_or_else(|| panic!("{:?} has no file name", entry.path()));

        match entry_kind {
            EntryKind::Dir => {
                if let Some(existing_entry) = self.children.get(entry_name) {
                    match existing_entry.kind {
                        Kind::Dir(_) => return Ok(()),
                        Kind::File {
                            content: _,
                            size: _,
                        } => {
                            return Err(IncoherentStructure(format!(
                                "Adding {}, cannot add a dir when file already exists",
                                entry.path()
                            ))
                            .into())
                        }
                        Kind::Link { target: _ } => {
                            return Err(IncoherentStructure(format!(
                                "Adding {}, cannot add a dir when link already exists",
                                entry.path()
                            ))
                            .into())
                        }
                    }
                };

                self.children.insert(
                    entry_name.into(),
                    Entry::new_dir(
                        entry_name.into(),
                        entry.uid(),
                        entry.gid(),
                        entry.mode(),
                        entry.mtime(),
                    ),
                );
                Ok(())
            }
            EntryKind::File(size, content_address) => {
                if self.children.contains_key(entry_name) {
                    return Err(IncoherentStructure(format!(
                        "Adding {}, cannot add a file when one already exists",
                        entry.path()
                    ))
                    .into());
                }
                self.children.insert(
                    entry_name.into(),
                    Entry::new_file(
                        entry_name.into(),
                        entry.uid(),
                        entry.gid(),
                        entry.mode(),
                        entry.mtime(),
                        content_address,
                        size.into_u64(),
                    ),
                );
                Ok(())
            }
            EntryKind::Link(target) => {
                if self.children.contains_key(entry_name) {
                    return Err(IncoherentStructure(format!(
                        "Adding {}, cannot add a link when one already exists",
                        entry.path()
                    ))
                    .into());
                }
                self.children.insert(
                    entry_name.into(),
                    Entry::new_link(
                        entry_name.into(),
                        entry.uid(),
                        entry.gid(),
                        entry.mode(),
                        entry.mtime(),
                        target,
                    ),
                );
                Ok(())
            }
        }
    }
}

fn set_idx(parent_dir: &mut DirEntry, idx: &mut impl Iterator<Item = u32>) {
    let mut to_visit = VecDeque::new();

    for child in parent_dir.children.values_mut() {
        to_visit.push_back((None, child));
    }

    let mut next_idx = || idx.next().map(jbk::EntryIdx::from);

    while let Some((parent_idx, entry)) = to_visit.pop_front() {
        let idx = next_idx();
        entry.set_idx(idx.unwrap(), parent_idx);
        if let Kind::Dir(d) = &mut entry.kind {
            for child in d.children.values_mut() {
                to_visit.push_back((idx, child));
            }
        }
    }
}

struct FlattenTree {
    to_visit: VecDeque<Entry>,
}

impl FlattenTree {
    fn new(dir_entry: DirEntry) -> Self {
        let mut to_visit = VecDeque::new();

        for child in dir_entry.children.into_values() {
            to_visit.push_back(child);
        }
        Self { to_visit }
    }
}

impl Iterator for FlattenTree {
    type Item = JbkEntry;

    fn next(&mut self) -> Option<JbkEntry> {
        self.to_visit.pop_front().map(|e| {
            let (jbk_entry, next_children) = JbkEntry::new(e);
            if let Some(next_children) = next_children {
                for child in next_children {
                    self.to_visit.push_back(child);
                }
            }
            jbk_entry
        })
    }
}

pub fn flat(mut tree: DirEntry) -> impl Iterator<Item = JbkEntry> {
    let mut idx = std::ops::RangeFrom { start: 0 };

    set_idx(&mut tree, &mut idx);

    FlattenTree::new(tree)
}
