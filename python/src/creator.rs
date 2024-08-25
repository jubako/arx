use std::rc::Rc;
use std::{path::PathBuf, sync::Arc};

use arx::create::{FsAdder, SimpleCreator};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// An Arx creator.
///
/// A creator is context manager and must be used as a context mananger.
///
/// > creator = libarx.Creator("new_archive.arx")
/// > with creator:
/// >    creator.add("foo/par")
#[pyclass(unsendable)]
pub struct Creator {
    started: bool,
    creator: Option<SimpleCreator>,
    outfile: PathBuf,
}

#[pymethods]
impl Creator {
    #[new]
    fn new(outfile: PathBuf) -> PyResult<Self> {
        Ok(Self {
            started: false,
            creator: Some(
                SimpleCreator::new(
                    &outfile,
                    jbk::creator::ConcatMode::OneFile,
                    Arc::new(()),
                    Rc::new(()),
                    jbk::creator::Compression::zstd(),
                )
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
            ),
            outfile,
        })
    }

    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        if slf.creator.is_none() {
            return Err(PyRuntimeError::new_err("Creator already finalized"));
        }
        if slf.started {
            return Err(PyRuntimeError::new_err("Creator already started"));
        }
        slf.started = true;
        Ok(slf)
    }

    fn __exit__(
        mut slf: PyRefMut<'_, Self>,
        _exc_type: PyObject,
        _exc_value: PyObject,
        _traceback: PyObject,
    ) -> PyResult<()> {
        if !slf.started {
            return Err(PyRuntimeError::new_err("Creator not started."));
        }
        slf.started = false;
        match slf.creator.take() {
            None => Err(PyRuntimeError::new_err("Creator already finalized")),
            Some(creator) => creator
                .finalize(&slf.outfile)
                .map_err(|e| PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Add the file `name` to the archive. `name` may be any type of file (directory, symlink, regular file).
    /// Directory are added recursively by default. This cane be avoided by setting `recursive` to `False`
    #[pyo3(signature=(path, recursive=true))]
    fn add(&mut self, path: PathBuf, recursive: bool) -> PyResult<()> {
        match self.creator.as_mut() {
            None => Err(PyRuntimeError::new_err("Creator already finalized")),
            Some(creator) => {
                if !self.started {
                    return Err(PyRuntimeError::new_err(
                        "add method must be used inside a context manager",
                    ));
                }
                let mut adder = FsAdder::new(creator, "".into());
                adder
                    .add_from_path(path, recursive)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
            }
        }
    }
}
