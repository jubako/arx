use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Copy)]
pub struct ContentAddress(pub(crate) jbk::ContentAddress);

impl From<jbk::ContentAddress> for ContentAddress {
    fn from(value: jbk::ContentAddress) -> Self {
        Self(value)
    }
}
