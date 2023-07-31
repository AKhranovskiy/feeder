use std::path::Path;

use lazy_static::lazy_static;

use numpy::IntoPyArray;
use pyo3::types::PyModule;
use pyo3::{Py, PyAny, Python};

use crate::types::{Data, PredictedLabels, PyModel};

pub(crate) struct PyVTable {
    num_gpus: Py<PyAny>,
    load: Py<PyAny>,
    predict: Py<PyAny>,
}

impl PyVTable {
    fn get() -> &'static PyVTable {
        lazy_static! {
            static ref PYVTABLE: PyVTable = {
                static SOURCE: &str = include_str!("source.py");

                Python::with_gil(|py| {
                    let source = PyModule::from_code(py, SOURCE, "source.py", "source")
                        .expect("Python source is loaded");

                    let attr =
                        |name: &str| source.getattr(name).expect("Attribute {name} is loaded");

                    PyVTable {
                        num_gpus: attr("num_gpus").into(),
                        load: attr("load_model").into(),
                        predict: attr("predict").into(),
                    }
                })
            };
        }
        &PYVTABLE
    }

    pub(crate) fn num_gpus() -> anyhow::Result<u32> {
        Python::with_gil(|py| Ok(Self::get().num_gpus.as_ref(py).call0()?.extract()?))
    }

    pub(crate) fn load<P>(path: &P) -> anyhow::Result<PyModel>
    where
        P: AsRef<Path>,
    {
        Python::with_gil(|py| {
            anyhow::Ok(Self::get().load.as_ref(py).call1((path.as_ref(),))?.into())
        })
    }

    pub(crate) fn predict(model: &PyModel, data: &Data) -> anyhow::Result<PredictedLabels> {
        Python::with_gil(|py| {
            let data = data
                .iter()
                .copied()
                .map(|x| f32::from(x) / 32768.0)
                .collect::<Vec<_>>()
                .into_pyarray(py);

            let model = model.as_ref(py);

            let pyarray: &numpy::PyArray2<f32> = Self::get()
                .predict
                .as_ref(py)
                .call1((model, data))?
                .extract()?;

            Ok(pyarray.readonly().as_array().to_owned())
        })
    }
}
