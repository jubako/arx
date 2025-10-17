#![allow(dead_code)]

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    ffi::{OsStr, OsString},
    fs::{read_dir, read_link, symlink_metadata, File, ReadDir},
    io::{self, BufReader, Read},
    ops::Deref,
    path::{Path, PathBuf},
};

struct ReadAsIter<R: Read>(BufReader<R>);

impl<R: Read> Iterator for ReadAsIter<R> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        let mut result: [u8; 1] = [0; 1];
        match self.0.read(&mut result).expect("read should succeed") {
            0 => None,
            1 => Some(result[0]),
            _ => unreachable!(),
        }
    }
}

impl<R: Read> From<R> for ReadAsIter<R> {
    fn from(value: R) -> Self {
        Self(BufReader::new(value))
    }
}

#[derive(Debug)]
pub enum TreeEntry {
    Dir(PathBuf),
    File(PathBuf),
    Link(PathBuf),
}

impl TreeEntry {
    fn new(p: &Path) -> io::Result<Self> {
        let metadata = symlink_metadata(p)?;
        if metadata.is_dir() {
            Ok(Self::Dir(p.to_path_buf()))
        } else if metadata.is_file() {
            Ok(Self::File(p.to_path_buf()))
        } else if metadata.is_symlink() {
            Ok(Self::Link(p.to_path_buf()))
        } else {
            unreachable!()
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            TreeEntry::Dir(p) => p,
            TreeEntry::File(p) => p,
            TreeEntry::Link(p) => p,
        }
    }

    pub fn file_name(&self) -> &OsStr {
        self.path().file_name().unwrap()
    }
}

impl PartialEq for TreeEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TreeEntry::File(a), TreeEntry::File(b)) => {
                let file_a: ReadAsIter<_> = File::open(a).expect("Open should succeed").into();
                let file_b: ReadAsIter<_> = File::open(b).expect("Open should succeed").into();
                file_a.cmp(file_b) == Ordering::Equal
            }
            (TreeEntry::Link(a), TreeEntry::Link(b)) => {
                let target_a = read_link(a).expect("Read_link should succeed");
                let target_b = read_link(b).expect("Read_link should succeed");
                target_a.cmp(&target_b) == Ordering::Equal
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
struct OrderByPath(TreeEntry);

impl OrderByPath {
    fn key(&self) -> &OsStr {
        self.0.file_name()
    }
}

impl Eq for OrderByPath {}

impl Ord for OrderByPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(other.key())
    }
}

impl PartialOrd for OrderByPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderByPath {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Deref for OrderByPath {
    type Target = TreeEntry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct EntryIterator(ReadDir);

impl EntryIterator {
    fn new(p: &Path) -> Self {
        Self(read_dir(p).unwrap())
    }
}

impl Iterator for EntryIterator {
    type Item = OrderByPath;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.0.next()?.expect("Iter dir should succeed");
        let file_type = next.file_type().unwrap();
        Some(OrderByPath(if file_type.is_dir() {
            TreeEntry::Dir(next.path())
        } else if file_type.is_file() {
            TreeEntry::File(next.path())
        } else if file_type.is_symlink() {
            TreeEntry::Link(next.path())
        } else {
            unreachable!()
        }))
    }
}

pub trait Differ {
    type Result;
    fn result(self) -> Self::Result;
    fn push(&mut self, entry_name: &OsStr);

    fn pop(&mut self);

    fn equal(&mut self, entry_tested: &TreeEntry, entry_ref: &TreeEntry);

    fn added(&mut self, entry: &TreeEntry);

    fn removed(&mut self, entry: &TreeEntry);

    fn diff(&mut self, entry_tested: &TreeEntry, entry_ref: &TreeEntry);
}

pub struct SimpleDiffer(bool);

impl SimpleDiffer {
    pub fn new() -> Self {
        SimpleDiffer(true)
    }
}

impl Differ for SimpleDiffer {
    type Result = bool;

    fn result(self) -> bool {
        self.0
    }
    fn push(&mut self, _entry_name: &OsStr) {}

    fn equal(&mut self, _entry_tested: &TreeEntry, _entry_ref: &TreeEntry) {}

    fn pop(&mut self) {}

    fn added(&mut self, _entry: &TreeEntry) {
        self.0 = false;
    }

    fn removed(&mut self, _entry: &TreeEntry) {
        self.0 = false;
    }

    fn diff(&mut self, _entry_tested: &TreeEntry, _entry_ref: &TreeEntry) {
        self.0 = false;
    }
}

pub struct ExceptionDiffer {
    current_path: PathBuf,
    exceptions: HashMap<PathBuf, ExistingExpected>,
    equal: bool,
}

impl<const N: usize> From<[(PathBuf, ExistingExpected); N]> for ExceptionDiffer {
    fn from(value: [(PathBuf, ExistingExpected); N]) -> Self {
        ExceptionDiffer::new(value.into())
    }
}

impl ExceptionDiffer {
    pub fn new(exceptions: HashMap<PathBuf, ExistingExpected>) -> Self {
        Self {
            current_path: PathBuf::new(),
            exceptions,
            equal: true,
        }
    }

    pub fn result(self) -> bool {
        self.equal
    }
}

impl Differ for ExceptionDiffer {
    type Result = bool;

    fn result(self) -> Self::Result {
        self.equal
    }
    fn push(&mut self, entry_name: &OsStr) {
        self.current_path.push(entry_name);
    }

    fn pop(&mut self) {
        self.current_path.pop();
    }

    fn equal(&mut self, _entry_tested: &TreeEntry, _entry_ref: &TreeEntry) {}

    fn added(&mut self, entry: &TreeEntry) {
        println!(
            "Added {} in {}",
            entry.path().display(),
            self.current_path.display()
        );
        self.equal = false;
    }

    fn removed(&mut self, entry: &TreeEntry) {
        println!(
            "Removed {} in {}",
            entry.path().display(),
            self.current_path.display()
        );
        self.equal = false;
    }

    fn diff(&mut self, entry_tested: &TreeEntry, entry_ref: &TreeEntry) {
        match self.exceptions.get(&self.current_path) {
            None => {
                println!(
                    "Entry {} is different than {}",
                    entry_tested.path().display(),
                    entry_ref.path().display()
                );
                self.equal = false;
            }
            Some(ExistingExpected::Existing) => {
                //Nothing to do
            }
            Some(ExistingExpected::Content(expected)) => {
                let found_content = std::fs::read(entry_tested.path()).unwrap();
                if found_content != *expected {
                    println!(
                        "Entry {} is different than expected content",
                        entry_tested.path().display()
                    );
                    self.equal = false;
                }
            }
            Some(ExistingExpected::Link(target)) => {
                let found_target = std::fs::read_link(entry_tested.path()).unwrap();
                if found_target != *target {
                    println!(
                        "Entry {} is different than expected content",
                        entry_tested.path().display()
                    );
                    self.equal = false;
                }
            }
        }
    }
}

fn diff(tested: &TreeEntry, reference: &TreeEntry, differ: &mut impl Differ) {
    match (tested, reference) {
        // two files or links, equal
        (TreeEntry::File(_), TreeEntry::File(_)) if tested == reference => {
            differ.equal(tested, reference)
        }
        (TreeEntry::Link(_), TreeEntry::Link(_)) if tested == reference => {
            differ.equal(tested, reference)
        }

        // Directories, we must compare
        (TreeEntry::Dir(path_tested), TreeEntry::Dir(path_ref)) => {
            let children_tested = EntryIterator::new(path_tested).collect::<BTreeSet<_>>();
            let children_ref = EntryIterator::new(path_ref).collect::<BTreeSet<_>>();
            for v_tested in children_tested.intersection(&children_ref) {
                let v_ref = children_ref.get(v_tested).expect("intersection to work");
                differ.push(v_tested.file_name());
                diff(v_tested, v_ref, differ);
                differ.pop();
            }
            for k in children_tested.difference(&children_ref) {
                differ.added(k);
            }
            for k in children_ref.difference(&children_tested) {
                differ.removed(k);
            }
        }

        // Different
        _ => differ.diff(tested, reference),
    }
}

pub enum ExistingExpected {
    Existing,
    Content(Vec<u8>),
    Link(OsString),
}

/// Compare two paths and return true if they are identical.
/// Identical means:
/// - Same kind
/// - Same content for files
/// - Same target for symlink
/// - Same entries for directory
///
/// `exceptions` is a HashMap of entry than can differ from source.
/// If value in the hashmap is `None`, we accept different content all the time.
/// If value is `Some(content)`, the content of the file must match `content`.
///
/// Please note that tested (generated by thing being tested) content comes first and
/// reference second. This is important when `exceptions` has values as we will load
/// file content in first (tested) tree.
pub fn tree_diff<D: Differ>(
    tested: impl AsRef<Path>,
    reference: impl AsRef<Path>,
    mut differ: D,
) -> std::io::Result<D::Result> {
    diff(
        &TreeEntry::new(tested.as_ref())?,
        &TreeEntry::new(reference.as_ref())?,
        &mut differ,
    );
    Ok(differ.result())
}

pub fn list_diff(
    tested_content: &[&str],
    reference: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> std::io::Result<bool> {
    let walker = walkdir::WalkDir::new(reference.as_ref());
    let walker = walker.into_iter();
    for entry in walker {
        let entry = entry?;
        if entry.path() == root.as_ref() {
            continue;
        }
        let entry_path = entry.path().strip_prefix(root.as_ref()).unwrap();
        let entry_path = relative_path::RelativePathBuf::from_path(&entry_path).unwrap();
        if !tested_content.contains(&entry_path.as_str()) {
            println!("{entry_path:?} is not in archive");
            println!("{tested_content:?}");
            return Ok(false);
        }
    }
    Ok(true)
}
