mod arx;
mod content_address;
mod creator;
mod entry;
mod iterator;
mod stream;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn libarx<'py>(_py: Python, m: Bound<'py, PyModule>) -> PyResult<()> {
    m.add_class::<arx::Arx>()?;
    m.add_class::<entry::Entry>()?;
    m.add_class::<content_address::ContentAddress>()?;
    m.add_class::<creator::Creator>()?;
    Ok(())
}
