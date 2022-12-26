use lazy_static::lazy_static;

use numpy::IntoPyArray;
use pyo3::types::PyModule;
use pyo3::{Py, PyAny, Python};

use crate::types::{Data, Labels, PredictedLabels, PyModel};

#[allow(dead_code)]
pub(crate) struct PyVTable {
    define: Py<PyAny>,
    load: Py<PyAny>,
    predict: Py<PyAny>,
    save: Py<PyAny>,
    train: Py<PyAny>,
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
                        define: attr("define_model").into(),
                        load: attr("load_model").into(),
                        predict: attr("predict").into(),
                        save: attr("save_model").into(),
                        train: attr("train_model").into(),
                    }
                })
            };
        }
        &PYVTABLE
    }

    pub(crate) fn define() -> anyhow::Result<PyModel> {
        Python::with_gil(|py| anyhow::Ok(Self::get().define.as_ref(py).call0()?.into()))
    }

    pub(crate) fn load(path: &str) -> anyhow::Result<PyModel> {
        Python::with_gil(|py| anyhow::Ok(Self::get().load.as_ref(py).call1((path,))?.into()))
    }

    pub(crate) fn save(model: &PyModel, path: &str) -> anyhow::Result<()> {
        Python::with_gil(|py| {
            Self::get().save.as_ref(py).call1((model, path))?;
            anyhow::Ok(())
        })
    }

    pub(crate) fn predict(model: &PyModel, data: &Data) -> anyhow::Result<PredictedLabels> {
        Python::with_gil(|py| {
            let data = data.clone().into_pyarray(py);
            let model = model.as_ref(py);
            let pyarray: &numpy::PyArray2<f32> = Self::get()
                .predict
                .as_ref(py)
                .call1((model, data))?
                .extract()?;
            Ok(pyarray.readonly().as_array().to_owned())
        })
    }

    pub(crate) fn train(model: &PyModel, data: &Data, labels: &Labels) -> anyhow::Result<PyModel> {
        Python::with_gil(|py| {
            anyhow::Ok(
                Self::get()
                    .train
                    .as_ref(py)
                    .call1((
                        model,
                        data.clone().into_pyarray(py),
                        labels.clone().into_pyarray(py),
                    ))?
                    .extract()?,
            )
        })
    }
}
