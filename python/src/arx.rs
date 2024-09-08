use std::sync::Arc;

use crate::iterator::EntryIter;

use super::content_address::ContentAddress;
use super::entry::Entry;
use arx::{FromPathErrorKind, PathBuf};
use jbk::reader::MayMissPack;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::{PyOSError, PyUnicodeDecodeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyUnicode;
use std::io::Read;

/// An Arx archive.
///
/// From an arx archive, you can access the entries in it.
///
/// # Accessing entrie is arx archive:
///
/// You can either:
///
/// ## Directly use `arx.get_entry("foo/bar/file.ext")` if you know the path of the entry.
///
/// > arx = libarx.Arx("archive.arx")
/// > entry = arx.get_entry("foo/bar/file.ext")
///
/// ## Iterate on the archive
///
/// > arx = libarx.Arx("archive.arx")
/// > for entry in arx:
/// >     ...
///
/// Arx archives contain a tree structure, so iterating on the archive will loop only on top level
/// entries. You will have to iterate on Directory entries to walk the tree structure.
#[pyclass]
pub struct Arx(Arc<arx::Arx>);

impl std::ops::Deref for Arx {
    type Target = arx::Arx;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Arx {
    fn new(arx: arx::Arx) -> Self {
        Self(Arc::new(arx))
    }

    pub(crate) fn get_content_rust<'py>(
        arx: &arx::Arx,
        py: Python<'py>,
        content: jbk::ContentAddress,
    ) -> PyResult<&'py pyo3::types::PyBytes> {
        let bytes = arx.container.get_bytes(content).unwrap();
        match bytes {
            MayMissPack::FOUND(bytes) => {
                let mut stream = bytes.stream();
                let read = |slice: &mut [u8]| {
                    stream.read_exact(slice).unwrap();
                    Ok(())
                };
                pyo3::types::PyBytes::new_with(py, bytes.size().into_usize(), read)
            }
            MayMissPack::MISSING(pack_info) => Err(PyOSError::new_err(format!(
                "Cannot found pack {}",
                pack_info.uuid
            ))),
        }
    }
}

#[pymethods]
impl Arx {
    #[new]
    fn py_new(path: &PyUnicode) -> PyResult<Self> {
        let path: std::path::PathBuf = path.extract()?;
        match arx::Arx::new(path) {
            Ok(a) => Ok(Self::new(a)),

            Err(_) => Err(PyValueError::new_err("Cannot create arx")),
        }
    }

    /// Get an entry for the given path
    fn get_entry(&self, path: std::path::PathBuf) -> PyResult<Entry> {
        let path = PathBuf::from_path(&path).map_err(|e| match e.kind() {
            FromPathErrorKind::NonRelative => {
                PyRuntimeError::new_err(format!("{} is not a relative path", path.display()))
            }
            FromPathErrorKind::NonUtf8 => {
                PyUnicodeDecodeError::new_err("Non utf8 char in path".to_string())
            }
            FromPathErrorKind::BadSeparator => PyRuntimeError::new_err("Invalid path separator"),
            _ => PyRuntimeError::new_err("Unknown error"),
        })?;
        match self.0.get_entry::<arx::FullBuilder>(&path) {
            Ok(e) => Ok(Entry::new(Arc::clone(&self.0), e)),
            Err(_) => Err(PyValueError::new_err("Cannot get entry")),
        }
    }

    /// Get the content associated to contentAddress
    fn get_content<'py>(
        &self,
        py: Python<'py>,
        content: ContentAddress,
    ) -> PyResult<&'py pyo3::types::PyBytes> {
        Self::get_content_rust(self, py, content.0)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<EntryIter>> {
        let iter = EntryIter::new_from_index(Arc::clone(&slf.0), &slf.0.root_index);
        Py::new(slf.py(), iter)
    }

    /// Extract the whole archive in
    #[pyo3(signature=(extract_path=std::path::PathBuf::from(".")))]
    fn extract(&self, extract_path: std::path::PathBuf) -> PyResult<()> {
        arx::extract_arx(&self.0, &extract_path, Default::default(), true, false)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}
