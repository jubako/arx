use std::{borrow::Cow, sync::Arc};

use arx::CommonEntry;
use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{content_address::ContentAddress, iterator::EntryIter};

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

    #[getter]
    fn idx(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.idx().into_u32(),
            arx::Entry::Link(e) => e.idx().into_u32(),
            arx::Entry::Dir(_, e) => e.idx().into_u32(),
        }
    }

    #[getter]
    fn path(&self) -> PyResult<String> {
        Ok(match &self.entry {
            arx::Entry::File(e) => String::from_utf8(e.path().clone()).unwrap(),
            arx::Entry::Link(e) => String::from_utf8(e.path().clone()).unwrap(),
            arx::Entry::Dir(_, e) => String::from_utf8(e.path().clone()).unwrap(),
        })
    }

    #[getter]
    fn parent(&self) -> PyResult<Option<u32>> {
        let parent = match &self.entry {
            arx::Entry::File(e) => e.parent(),
            arx::Entry::Link(e) => e.parent(),
            arx::Entry::Dir(_, e) => e.parent(),
        };
        Ok(parent.map(|p| p.into_u32()))
    }

    #[getter]
    fn owner(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.owner(),
            arx::Entry::Link(e) => e.owner(),
            arx::Entry::Dir(_, e) => e.owner(),
        }
    }

    #[getter]
    fn group(&self) -> u32 {
        match &self.entry {
            arx::Entry::File(e) => e.group(),
            arx::Entry::Link(e) => e.group(),
            arx::Entry::Dir(_, e) => e.group(),
        }
    }

    #[getter]
    fn rights(&self) -> u8 {
        match &self.entry {
            arx::Entry::File(e) => e.rights(),
            arx::Entry::Link(e) => e.rights(),
            arx::Entry::Dir(_, e) => e.rights(),
        }
    }

    #[getter]
    fn mtime(&self) -> u64 {
        match &self.entry {
            arx::Entry::File(e) => e.mtime(),
            arx::Entry::Link(e) => e.mtime(),
            arx::Entry::Dir(_, e) => e.mtime(),
        }
    }

    fn is_file(&self) -> bool {
        if let arx::Entry::File(_) = &self.entry {
            true
        } else {
            false
        }
    }

    fn is_link(&self) -> bool {
        if let arx::Entry::Link(_) = &self.entry {
            true
        } else {
            false
        }
    }

    fn is_dir(&self) -> bool {
        if let arx::Entry::Dir(_, _) = &self.entry {
            true
        } else {
            false
        }
    }

    fn get_content_address(&self) -> PyResult<ContentAddress> {
        match &self.entry {
            arx::Entry::File(f) => Ok(f.content().into()),
            _ => Err(PyValueError::new_err("Not a file")),
        }
    }

    fn get_content<'py>(&self, py: Python<'py>) -> PyResult<&'py pyo3::types::PyBytes> {
        match &self.entry {
            arx::Entry::File(f) => super::arx::Arx::get_content_rust(&self.arx, py, f.content()),
            _ => Err(PyValueError::new_err("Not a file")),
        }
    }

    fn get_target(&self) -> PyResult<Cow<[u8]>> {
        match &self.entry {
            arx::Entry::Link(l) => Ok(l.target().into()),
            _ => Err(PyValueError::new_err("Not a link")),
        }
    }

    fn first_child(&self) -> PyResult<u32> {
        match &self.entry {
            arx::Entry::Dir(range, _) => Ok(range.begin().into_u32()),
            _ => Err(PyValueError::new_err("Not a dir")),
        }
    }

    fn nb_children(&self) -> PyResult<u32> {
        match &self.entry {
            arx::Entry::Dir(range, _) => Ok(range.size().into_u32()),
            _ => Err(PyValueError::new_err("Not a dir")),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<EntryIter>> {
        match &slf.entry {
            arx::Entry::Dir(range, _) => {
                let iter = EntryIter::new_from_range(Arc::clone(&slf.arx), range);
                Py::new(slf.py(), iter)
            }
            _ => Err(PyValueError::new_err("Not a dir")),
        }
    }
}
