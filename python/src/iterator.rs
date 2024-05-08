use std::sync::Arc;

use super::entry::Entry;
use jbk::{
    reader::{Index, Range},
    EntryIdx, EntryRange,
};
use pyo3::{exceptions::PyRuntimeError, prelude::*};

#[pyclass]
pub struct EntryIter {
    arx: Arc<arx::Arx>,
    start: EntryIdx,
    end: EntryIdx,
}

impl EntryIter {
    pub fn new_from_range(arx: Arc<arx::Arx>, range: &EntryRange) -> Self {
        Self {
            arx,
            start: range.begin(),
            end: range.end(),
        }
    }
    pub fn new_from_index(arx: Arc<arx::Arx>, range: &Index) -> Self {
        Self {
            arx,
            start: range.offset(),
            end: range.offset() + range.count(),
        }
    }
}

#[pymethods]
impl EntryIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Entry>> {
        if slf.start == slf.end {
            return Ok(None);
        }
        let ret = Entry::new(
            Arc::clone(&slf.arx),
            slf.arx
                .get_entry_at_idx::<arx::FullBuilder>(slf.start)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
        );
        slf.start += 1;
        Ok(Some(ret))
    }
}
