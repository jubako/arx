use std::sync::Arc;

use crate::iterator::EntryIter;

use super::content_address::ContentAddress;
use super::entry::Entry;
use arx::PathBuf;
use jbk::reader::MayMissPack;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyUnicode;
use std::io::Read;

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
                let read = |slice: &mut [u8]| Ok(stream.read_exact(slice).unwrap());
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
        let path = PathBuf::from_path(path).map_err(|e| PyTypeError::new_err(e.to_string()))?;
        match self.0.get_entry::<arx::FullBuilder>(&path) {
            Ok(e) => Ok(Entry::new(Arc::clone(&self.0), e)),
            Err(_) => Err(PyValueError::new_err("Cannot get entry")),
        }
    }

    fn get_content<'py>(
        &self,
        py: Python<'py>,
        content: ContentAddress,
    ) -> PyResult<&'py pyo3::types::PyBytes> {
        Self::get_content_rust(&self, py, content.0)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<EntryIter>> {
        let iter = EntryIter::new_from_index(Arc::clone(&slf.0), &slf.0.root_index);
        Py::new(slf.py(), iter)
    }

    fn extract(&self, extract_path: std::path::PathBuf) -> PyResult<()> {
        arx::extract_arx(&self.0, &extract_path, Default::default(), false)
            .map_err(|_e| PyValueError::new_err("oups"))
    }
}
