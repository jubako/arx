use std::{borrow::Cow, sync::Arc};

use arx::CommonEntry;
use pyo3::{
    exceptions::{PyRuntimeError, PyTypeError},
    prelude::*,
};

use crate::{content_address::ContentAddress, iterator::EntryIter};

/// An entry i an arx archive.
///
/// Can be a directory, a file or a link.
#[pyclass]
pub struct Entry {
    arx: Arc<arx::Arx>,
    entry: arx::FullEntry,
}

impl Entry {
    pub fn new(arx: Arc<arx::Arx>, entry: arx::FullEntry) -> Self {
        Self { arx, entry }
    }
}

#[pymethods]
impl Entry {
    fn __repr__(&self) -> String {
        match &self.entry {
            arx::Entry::File(e) => {
                format!("File({})", String::from_utf8(e.path().clone()).unwrap())
            }
            arx::Entry::Link(e) => {
                format!("Link({})", String::from_utf8(e.path().clone()).unwrap())
            }
            arx::Entry::Dir(_, e) => {
                format!("Dir({})", String::from_utf8(e.path().clone()).unwrap())
            }
        }
    }

    /// The index of the current entry
    #[getter]
    fn idx(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.idx().into_u32(),
            arx::Entry::Link(e) => e.idx().into_u32(),
            arx::Entry::Dir(_, e) => e.idx().into_u32(),
        }
    }

    /// The path (relative to its parent entry)
    #[getter]
    fn path(&self) -> PyResult<String> {
        Ok(match &self.entry {
            arx::Entry::File(e) => String::from_utf8(e.path().clone()).unwrap(),
            arx::Entry::Link(e) => String::from_utf8(e.path().clone()).unwrap(),
            arx::Entry::Dir(_, e) => String::from_utf8(e.path().clone()).unwrap(),
        })
    }

    /// The index of the parent entry.
    #[getter]
    fn parent(&self) -> PyResult<Option<Entry>> {
        let parent_idx = match &self.entry {
            arx::Entry::File(e) => e.parent(),
            arx::Entry::Link(e) => e.parent(),
            arx::Entry::Dir(_, e) => e.parent(),
        };
        match parent_idx {
            None => Ok(None),
            Some(i) => {
                let arx_entry = self
                    .arx
                    .get_entry_at_idx::<arx::FullBuilder>(i)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                let entry = Entry::new(self.arx.clone(), arx_entry);
                Ok(Some(entry))
            }
        }
    }

    /// The owner (int) of the entry.
    #[getter]
    fn owner(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.owner(),
            arx::Entry::Link(e) => e.owner(),
            arx::Entry::Dir(_, e) => e.owner(),
        }
    }

    /// The group (int) of the entry.
    #[getter]
    fn group(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.group(),
            arx::Entry::Link(e) => e.group(),
            arx::Entry::Dir(_, e) => e.group(),
        }
    }

    /// The rigths (int) of the entry.
    #[getter]
    fn rights(&self) -> u8 {
        match &self.entry {
            arx::Entry::File(e) => e.rights(),
            arx::Entry::Link(e) => e.rights(),
            arx::Entry::Dir(_, e) => e.rights(),
        }
    }

    /// The modification time of the entry.
    #[getter]
    fn mtime(&self) -> u64 {
        match &self.entry {
            arx::Entry::File(e) => e.mtime(),
            arx::Entry::Link(e) => e.mtime(),
            arx::Entry::Dir(_, e) => e.mtime(),
        }
    }

    /// Return True if the entry is a file entry
    fn is_file(&self) -> bool {
        matches!(&self.entry, arx::Entry::File(_))
    }

    /// Return True if the entry is a link entry
    fn is_link(&self) -> bool {
        matches!(&self.entry, arx::Entry::Link(_))
    }

    /// Return True if the entry is a dir entry
    fn is_dir(&self) -> bool {
        matches!(&self.entry, arx::Entry::Dir(_, _))
    }

    /// Get the content address of the file entry.
    ///
    /// Raise an exception if entry is not a file.
    fn get_content_address(&self) -> PyResult<ContentAddress> {
        match &self.entry {
            arx::Entry::File(f) => Ok(f.content().into()),
            _ => Err(PyTypeError::new_err("Not a file")),
        }
    }

    /// Get the content of the file entry.
    ///
    /// Raise an exception if entry is not a file.
    fn get_content<'py>(&self, py: Python<'py>) -> PyResult<&'py pyo3::types::PyBytes> {
        match &self.entry {
            arx::Entry::File(f) => super::arx::Arx::get_content_rust(&self.arx, py, f.content()),
            _ => Err(PyTypeError::new_err("Not a file")),
        }
    }

    /// Get the link target of the link entry.
    ///
    /// Raise an exception if entry is not a link.
    fn get_target(&self) -> PyResult<Cow<[u8]>> {
        match &self.entry {
            arx::Entry::Link(l) => Ok(l.target().into()),
            _ => Err(PyTypeError::new_err("Not a link")),
        }
    }

    /// Get the index of the first child of the dir entry.
    ///
    /// Raise an exception if entry is not a directory.
    fn first_child(&self) -> PyResult<u32> {
        match &self.entry {
            arx::Entry::Dir(range, _) => Ok(range.begin().into_u32()),
            _ => Err(PyTypeError::new_err("Not a dir")),
        }
    }

    /// Get the number of children in the dir entry.
    ///
    /// Raise an exception if entry is not a directory.
    fn nb_children(&self) -> PyResult<u32> {
        match &self.entry {
            arx::Entry::Dir(range, _) => Ok(range.size().into_u32()),
            _ => Err(PyTypeError::new_err("Not a dir")),
        }
    }

    /// Iter on all child entries in the dir entry.
    ///
    /// Raise an exception if entry is not a directory.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<EntryIter>> {
        match &slf.entry {
            arx::Entry::Dir(range, _) => {
                let iter = EntryIter::new_from_range(Arc::clone(&slf.arx), range);
                Py::new(slf.py(), iter)
            }
            _ => Err(PyTypeError::new_err("Not a dir")),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.idx() == other.idx()
    }
}
