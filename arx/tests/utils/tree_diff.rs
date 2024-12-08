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
enum Entry {
    Dir(PathBuf),
    File(PathBuf),
    Link(PathBuf),
}

impl Entry {
    fn new(p: &Path) -> io::Result<Self> {
        let metadata = symlink_metadata(p)?;
        if metadata.is_dir() {
            Ok(Self::Dir(p.to_path_buf()))
        } else if metadata.is_file() {
            Ok(Self::File(p.to_path_buf()))
        } else if metadata.is_symlink() {
            Ok(Self::File(p.to_path_buf()))
        } else {
            unreachable!()
        }
    }

    fn path(&self) -> &Path {
        match self {
            Entry::Dir(p) => p,
            Entry::File(p) => p,
            Entry::Link(p) => p,
        }
    }

    fn file_name(&self) -> &OsStr {
        self.path().file_name().unwrap()
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Entry::File(a), Entry::File(b)) => {
                let file_a: ReadAsIter<_> = File::open(a).expect("Open should succeed").into();
                let file_b: ReadAsIter<_> = File::open(b).expect("Open should succeed").into();
                file_a.cmp(file_b) == Ordering::Equal
            }
            (Entry::Link(a), Entry::Link(b)) => {
                let target_a = read_link(a).expect("Read_link should succeed");
                let target_b = read_link(b).expect("Read_link should succeed");
                target_a.cmp(&target_b) == Ordering::Equal
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
struct OrderByPath(Entry);

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
        self.key().partial_cmp(other.key())
    }
}

impl PartialEq for OrderByPath {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Deref for OrderByPath {
    type Target = Entry;
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
            Entry::Dir(next.path())
        } else if file_type.is_file() {
            Entry::File(next.path())
        } else if file_type.is_symlink() {
            Entry::Link(next.path())
        } else {
            unreachable!()
        }))
    }
}

struct ExceptionMatcher {
    current_path: PathBuf,
    exceptions: HashMap<PathBuf, ExistingExpected>,
    equal: bool,
}

impl ExceptionMatcher {
    fn new(exceptions: HashMap<PathBuf, ExistingExpected>) -> Self {
        Self {
            current_path: PathBuf::new(),
            exceptions,
            equal: true,
        }
    }

    fn result(self) -> bool {
        self.equal
    }

    fn push(&mut self, entry_name: &OsStr) {
        self.current_path.push(entry_name);
    }

    fn pop(&mut self) {
        self.current_path.pop();
    }

    fn added(&mut self, entry: &Entry) {
        println!(
            "Added {} in {}",
            entry.path().display(),
            self.current_path.display()
        );
        self.equal = false;
    }

    fn removed(&mut self, entry: &Entry) {
        println!(
            "Removed {} in {}",
            entry.path().display(),
            self.current_path.display()
        );
        self.equal = false;
    }

    fn diff(&mut self, entry_tested: &Entry, entry_ref: &Entry) {
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

fn diff(tested: &Entry, reference: &Entry, matcher: &mut ExceptionMatcher) {
    match (tested, reference) {
        // two files or links, equal
        (Entry::File(_), Entry::File(_)) if tested == reference => {}
        (Entry::Link(_), Entry::Link(_)) if tested == reference => {}

        // Directories, we must compare
        (Entry::Dir(path_tested), Entry::Dir(path_ref)) => {
            let children_tested = EntryIterator::new(&path_tested).collect::<BTreeSet<_>>();
            let children_ref = EntryIterator::new(&path_ref).collect::<BTreeSet<_>>();
            for v_tested in children_tested.intersection(&children_ref) {
                let v_ref = children_ref.get(v_tested).expect("intersection to work");
                matcher.push(v_tested.file_name());
                diff(&v_tested, &v_ref, matcher);
                matcher.pop();
            }
            for k in children_tested.difference(&children_ref) {
                matcher.added(k);
            }
            for k in children_ref.difference(&children_tested) {
                matcher.removed(k);
            }
        }

        // Different
        _ => matcher.diff(tested, reference),
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
pub fn tree_diff(
    tested: impl AsRef<Path>,
    reference: impl AsRef<Path>,
    exceptions: HashMap<PathBuf, ExistingExpected>,
) -> std::io::Result<bool> {
    let mut matcher = ExceptionMatcher::new(exceptions);
    diff(
        &Entry::new(tested.as_ref())?,
        &Entry::new(reference.as_ref())?,
        &mut matcher,
    );
    Ok(matcher.result())
}
