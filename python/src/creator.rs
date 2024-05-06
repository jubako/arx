use std::rc::Rc;
use std::{path::PathBuf, sync::Arc};

use arx::create::{FsAdder, SimpleCreator};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

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
                .map_err(|_| PyValueError::new_err("Cannot create creator"))?,
            ),
            outfile,
        })
    }

    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        if slf.creator.is_none() {
            return Err(PyValueError::new_err("Creator already finalized"));
        }
        if slf.started {
            return Err(PyValueError::new_err("Creator already started"));
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
            return Err(PyValueError::new_err("Creator not started."));
        }
        slf.started = false;
        match slf.creator.take() {
            None => Err(PyValueError::new_err("Creator already finalized")),
            Some(creator) => creator
                .finalize(&slf.outfile)
                .map_err(|_| PyValueError::new_err("Cannot Finalize")),
        }
    }

    fn add(&mut self, path: PathBuf, recursive: bool) -> PyResult<()> {
        match self.creator.as_mut() {
            None => Err(PyValueError::new_err("Creator already finalized")),
            Some(creator) => {
                if !self.started {
                    return Err(PyValueError::new_err(
                        "add method must be used inside a context manager",
                    ));
                }
                let mut adder = FsAdder::new(creator, "".into());
                adder
                    .add_from_path(path, recursive)
                    .map_err(|_| PyValueError::new_err("Cannot add file/dir"))
            }
        }
    }
}
