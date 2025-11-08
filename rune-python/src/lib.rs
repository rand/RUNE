//! Python bindings for RUNE using PyO3

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::exceptions::PyValueError;
use rune_core::{
    RUNEEngine as CoreEngine,
    Request, RequestBuilder,
    Principal, Action, Resource,
    Value, Decision,
};
use std::sync::Arc;
use std::collections::HashMap;

/// Python wrapper for RUNE engine
#[pyclass(name = "RUNE")]
struct PythonRUNE {
    engine: Arc<CoreEngine>,
}

#[pymethods]
impl PythonRUNE {
    /// Create a new RUNE engine
    #[new]
    #[pyo3(signature = (config_path=None))]
    fn new(config_path: Option<String>) -> PyResult<Self> {
        let engine = CoreEngine::new();

        if let Some(path) = config_path {
            // TODO: Load configuration
            // engine.load_configuration(&path)
            //     .map_err(|e| PyValueError::new_err(format!("Failed to load config: {}", e)))?;
        }

        Ok(PythonRUNE {
            engine: Arc::new(engine),
        })
    }

    /// Authorize a request
    #[pyo3(signature = (action, principal=None, resource=None, **kwargs))]
    fn authorize(
        &self,
        action: String,
        principal: Option<String>,
        resource: Option<String>,
        kwargs: Option<&PyDict>,
    ) -> PyResult<bool> {
        // Build request
        let principal = Principal::agent(principal.unwrap_or_else(|| "default".to_string()));
        let action = Action::new(action);
        let resource = Resource::file(resource.unwrap_or_else(|| "/".to_string()));

        let mut request = Request::new(principal, action, resource);

        // Add context from kwargs
        if let Some(dict) = kwargs {
            for (key, value) in dict.iter() {
                let key_str = key.extract::<String>()?;
                let val = python_to_value(value)?;
                request = request.with_context(key_str, val);
            }
        }

        // Evaluate
        let result = self.engine
            .authorize(&request)
            .map_err(|e| PyValueError::new_err(format!("Authorization failed: {}", e)))?;

        Ok(result.decision.is_permitted())
    }

    /// Batch authorize multiple requests
    fn authorize_batch(&self, requests: &PyList) -> PyResult<Vec<bool>> {
        let mut results = Vec::new();

        for item in requests.iter() {
            let dict = item.downcast::<PyDict>()?;

            let action = dict
                .get_item("action")?
                .ok_or_else(|| PyValueError::new_err("Missing 'action' field"))?
                .extract::<String>()?;

            let principal = dict
                .get_item("principal")?
                .map(|p| p.extract::<String>())
                .transpose()?
                .unwrap_or_else(|| "default".to_string());

            let resource = dict
                .get_item("resource")?
                .map(|r| r.extract::<String>())
                .transpose()?
                .unwrap_or_else(|| "/".to_string());

            let request = Request::new(
                Principal::agent(principal),
                Action::new(action),
                Resource::file(resource),
            );

            let result = self.engine
                .authorize(&request)
                .map_err(|e| PyValueError::new_err(format!("Authorization failed: {}", e)))?;

            results.push(result.decision.is_permitted());
        }

        Ok(results)
    }

    /// Add a fact to the engine
    fn add_fact(&self, predicate: String, args: Vec<PyObject>) -> PyResult<()> {
        let values: Result<Vec<Value>, _> = Python::with_gil(|py| {
            args.iter()
                .map(|obj| python_to_value(obj.as_ref(py)))
                .collect()
        });

        let values = values?;
        self.engine.add_fact(predicate, values);

        Ok(())
    }

    /// Clear the cache
    fn clear_cache(&self) -> PyResult<()> {
        self.engine.clear_cache();
        Ok(())
    }

    /// Get cache statistics
    fn cache_stats(&self) -> PyResult<HashMap<String, f64>> {
        let stats = self.engine.cache_stats();
        let mut result = HashMap::new();
        result.insert("size".to_string(), stats.size as f64);
        result.insert("hit_rate".to_string(), stats.hit_rate);
        Ok(result)
    }
}

/// Convert Python value to RUNE Value
fn python_to_value(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Integer(i))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::string(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let values: Result<Vec<Value>, _> = list
            .iter()
            .map(python_to_value)
            .collect();
        Ok(Value::array(values?))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = std::collections::BTreeMap::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let val = python_to_value(value)?;
            map.insert(key_str, val);
        }
        Ok(Value::object(map))
    } else {
        Err(PyValueError::new_err("Unsupported value type"))
    }
}

/// Decorator for requiring permission
#[pyclass]
struct RequirePermission {
    engine: Arc<CoreEngine>,
    action: String,
}

#[pymethods]
impl RequirePermission {
    #[new]
    fn new(engine: &PythonRUNE, action: String) -> Self {
        RequirePermission {
            engine: engine.engine.clone(),
            action,
        }
    }

    fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        // This would implement the decorator logic
        // For now, return the function unchanged
        Ok(func)
    }
}

/// Python module initialization
#[pymodule]
fn rune_python(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PythonRUNE>()?;
    m.add_class::<RequirePermission>()?;

    // Add version constant
    m.add("__version__", rune_core::VERSION)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_bindings() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = PythonRUNE::new(None).unwrap();

            // Test basic authorization
            let result = engine.authorize(
                "read".to_string(),
                Some("agent-1".to_string()),
                Some("/tmp/test.txt".to_string()),
                None,
            ).unwrap();

            // Default implementation permits everything for now
            assert!(result);
        });
    }
}