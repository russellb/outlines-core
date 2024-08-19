use pyo3::pymodule;

#[pymodule]
mod _lib {
    use outlines_core as core_lib;
    use pyo3::{pyfunction, PyResult};

    #[pyfunction]
    fn hello() -> PyResult<String> {
        Ok(core_lib::hello())
    }
}
