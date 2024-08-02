/// A Python wrapper to `GraphDiff<Uuid, drisk_api::NodeUpdate>`.
use crate::{bytes::graph_diff_to_bytes, diff::GraphDiff, node_update::NodeUpdate};
use pyo3::{
    exceptions::PyException,
    prelude::*,
    types::{PyAny, PyBytes, PyDict, PyList},
};
use uuid::Uuid;

pub struct PyNodeUpdate {
    pub label: Option<String>,
    pub url: Option<String>,
    pub size: Option<f32>,
    pub red: Option<u8>,
    pub green: Option<u8>,
    pub blue: Option<u8>,
    pub show_label: Option<bool>,
}

impl<'s> FromPyObject<'s> for PyNodeUpdate {
    fn extract(ob: &'s PyAny) -> PyResult<Self> {
        let dict = ob.downcast::<PyDict>()?;

        // helper macro to reduce code to go from PyAny -> T
        macro_rules! extract_field {
            ($lab: expr, $ty: ty) => {
                match dict.get_item($lab) {
                    Ok(Some(item)) => item.extract::<$ty>().map(Some).map_err(PyErr::from),
                    Ok(None) => Ok(None),
                    Err(e) => Err(e),
                }
            };
        }

        Ok(PyNodeUpdate {
            label: extract_field!("label", String)?,
            url: extract_field!("url", String)?,
            size: extract_field!("size", f32)?,
            red: extract_field!("red", u8)?,
            green: extract_field!("green", u8)?,
            blue: extract_field!("blue", u8)?,
            show_label: extract_field!("show_label", bool)?,
        })
    }
}

impl ToPyObject for NodeUpdate {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        let dict = PyDict::new_bound(py);

        macro_rules! set_item {
            ($key: expr, $val: expr) => {
                if let Some(val) = $val {
                    let _ = dict.set_item($key, val);
                }
            };
        }
        set_item!("label", &self.label);
        set_item!("url", &self.url);
        set_item!("size", self.size);
        set_item!("red", self.red);
        set_item!("green", self.green);
        set_item!("blue", self.blue);
        set_item!("show_label", self.show_label);

        dict.into()
    }
}

impl From<PyNodeUpdate> for NodeUpdate {
    fn from(node_update: PyNodeUpdate) -> Self {
        NodeUpdate {
            label: node_update.label,
            url: node_update.url,
            size: node_update.size,
            red: node_update.red,
            green: node_update.green,
            blue: node_update.blue,
            show_label: node_update.show_label,
        }
    }
}

fn pybytes_to_uuid(bytes: &Bound<'_, PyAny>) -> PyResult<Uuid> {
    let bytes = bytes.downcast::<PyBytes>()?.as_bytes();
    if bytes.len() != 16 {
        return Err(PyException::new_err("Expected 16 bytes."));
    }
    Uuid::from_slice(bytes).map_err(|_| PyException::new_err("Failed to parse UUID."))
}

#[derive(FromPyObject)]
pub struct PyUuid(#[pyo3(from_py_with = "pybytes_to_uuid")] Uuid);

#[pyclass]
pub struct PyGraphDiff(GraphDiff<Uuid, NodeUpdate>);

#[pymethods]
impl PyGraphDiff {
    #[new]
    fn new() -> Self {
        PyGraphDiff(GraphDiff::<_, _, f32>::new())
    }

    fn new_or_updated_nodes<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new_bound(py);
        for (id, node) in self.0.new_or_updated_nodes() {
            dict.set_item(id.to_string(), node.to_object(py))?;
        }
        PyResult::Ok(dict)
    }

    fn deleted_nodes<'a>(&self, py: Python<'a>) -> Bound<'a, PyList> {
        let ids = self
            .0
            .deleted_nodes()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let list = PyList::new_bound(py, ids);
        list
    }

    fn new_or_updated_edges<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new_bound(py);
        for (from, tos) in self.0.new_or_updated_edges() {
            if tos.is_empty() {
                continue;
            }
            let tos_dict = PyDict::new_bound(py);
            for (to, weight) in tos {
                tos_dict.set_item(to.to_string(), weight)?;
            }
            dict.set_item(from.to_string(), tos_dict)?;
        }
        PyResult::Ok(dict)
    }

    fn deleted_edges<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new_bound(py);
        for (from, tos) in self.0.deleted_edges() {
            if tos.is_empty() {
                continue;
            }
            let tos_list =
                PyList::new_bound(py, tos.iter().map(|to| to.to_string()).collect::<Vec<_>>());
            dict.set_item(from.to_string(), tos_list)?;
        }
        PyResult::Ok(dict)
    }

    fn num_nodes(&self) -> usize {
        self.0.nodes.get_new_or_updated().len() + self.0.nodes.get_deleted().len()
    }

    fn num_edges(&self) -> usize {
        self.0.edges.get_new_or_updated().len() + self.0.edges.get_deleted().len()
    }

    fn add_node(&mut self, id: PyUuid, update: PyNodeUpdate) {
        self.0.add_or_update_node(&id.0, update.into());
    }

    fn delete_node(&mut self, id: PyUuid) {
        self.0.delete_node(id.0);
    }

    fn add_edge(&mut self, from: PyUuid, to: PyUuid, weight: f32) {
        let _ = self.0.add_edge(&from.0, &to.0, weight);
    }

    fn delete_edge(&mut self, from: PyUuid, to: PyUuid) {
        self.0.delete_edge(&from.0, &to.0);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = graph_diff_to_bytes(&self.0)
            .map_err(|_| PyException::new_err("Failed to serialize graph diff."))?;
        Ok(PyBytes::new_bound(py, &bytes))
    }

    #[staticmethod]
    fn from_bytes(bytes: &Bound<'_, PyBytes>) -> PyResult<PyGraphDiff> {
        let graph_diff = crate::bytes::bytes_to_graph_diff(bytes.as_bytes())
            .map_err(|_| PyException::new_err("Failed to deserialize graph diff."))?;
        Ok(PyGraphDiff(graph_diff))
    }
}

#[pymodule]
pub fn drisk_api(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGraphDiff>()?;
    Ok(())
}
