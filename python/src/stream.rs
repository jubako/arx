use jbk::reader::ByteStream;
use pyo3::prelude::*;
use std::io::Read;

#[pyclass]
pub struct Stream(pub ByteStream);

#[pymethods]
impl Stream {
    /// Read `size` bytes from the stream.
    ///
    /// Returned `bytes` may be shorter than `size` if data left to be read is smaller than requested.
    fn read<'py>(
        &mut self,
        py: Python<'py>,
        size: usize,
    ) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
        let size = std::cmp::min(size, self.0.size_left() as usize);
        let read_fn = |slice: &mut [u8]| {
            self.0.read_exact(slice).unwrap();
            Ok(())
        };
        pyo3::types::PyBytes::new_bound_with(py, size, read_fn)
    }

    /// Get the full size of the stream.
    fn size(&self) -> u64 {
        self.0.size()
    }

    /// Get the size of the data left to read.
    ///
    /// Equivalent to `size() - tell()`
    fn size_left(&self) -> u64 {
        self.0.size_left()
    }

    /// Get the current offset (already read data) of the stream.
    fn tell(&self) -> u64 {
        self.0.offset()
    }
}
