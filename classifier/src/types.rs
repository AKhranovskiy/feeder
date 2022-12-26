pub type Data = ndarray::Array4<f64>;
pub type PredictedLabels = ndarray::Array2<f32>;
pub type Labels = ndarray::Array1<u32>;

pub(crate) type PyModel = pyo3::Py<pyo3::PyAny>;
