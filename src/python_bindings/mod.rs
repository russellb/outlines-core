use crate::index::{FSMInfo, Index};
use crate::json_schema;
use crate::prelude::*;
use crate::regex::get_token_transition_keys;
use crate::regex::get_vocabulary_transition_keys;
use crate::regex::state_scan_tokens;
use crate::regex::walk_fsm;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3::{wrap_pyfunction, Python};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[pyclass(module = "outlines_core.fsm.outlines_core_rs", name = "FSMInfo")]
pub struct PyFSMInfo {
    #[pyo3(get)]
    initial: State,
    #[pyo3(get)]
    finals: HashSet<State>,
    #[pyo3(get)]
    transitions: HashMap<(State, TransitionKey), State>,
    #[pyo3(get)]
    alphabet_anything_value: TransitionKey,
    #[pyo3(get)]
    alphabet_symbol_mapping: HashMap<String, TransitionKey>,
}

impl From<FSMInfo> for PyFSMInfo {
    fn from(fsm_info: FSMInfo) -> Self {
        PyFSMInfo {
            initial: fsm_info.initial,
            finals: fsm_info.finals,
            transitions: fsm_info.transitions,
            alphabet_anything_value: fsm_info.alphabet_anything_value,
            alphabet_symbol_mapping: fsm_info.alphabet_symbol_mapping,
        }
    }
}

// FIXME: could be costly, confirm if FSMInfo will actually be part of the interface
impl From<PyFSMInfo> for FSMInfo {
    fn from(fsm_info: PyFSMInfo) -> Self {
        FSMInfo {
            initial: fsm_info.initial,
            finals: fsm_info.finals.clone(),
            transitions: fsm_info.transitions.clone(),
            alphabet_anything_value: fsm_info.alphabet_anything_value,
            alphabet_symbol_mapping: fsm_info.alphabet_symbol_mapping.clone(),
        }
    }
}

#[pymethods]
impl PyFSMInfo {
    #[new]
    fn new(
        initial: State,
        finals: HashSet<State>,
        transitions: HashMap<(State, TransitionKey), State>,
        alphabet_anything_value: TransitionKey,
        alphabet_symbol_mapping: HashMap<String, TransitionKey>,
    ) -> Self {
        FSMInfo::new(
            initial,
            finals,
            transitions,
            alphabet_anything_value,
            alphabet_symbol_mapping,
        )
        .into()
    }

    fn __getstate__(&self, py: Python) -> PyResult<PyObject> {
        let data = serde_json::to_string(&self)
            .map_err(|e| PyException::new_err(format!("Failed to pickle FSMInfo: {}", e)))?;
        Ok(PyBytes::new_bound(py, data.as_bytes()).to_object(py))
    }

    fn __setstate__(&mut self, py: Python, state: PyObject) -> PyResult<()> {
        match state.extract::<&[u8]>(py) {
            Ok(s) => {
                *self = serde_json::from_slice(s).map_err(|e| {
                    PyException::new_err(format!("Failed to unpickle FSMInfo: {}", e))
                })?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn __getnewargs__(
        &self,
    ) -> PyResult<(
        State,
        HashSet<State>,
        HashMap<(State, TransitionKey), State>,
        TransitionKey,
        HashMap<String, TransitionKey>,
    )> {
        Ok((
            self.initial,
            self.finals.clone(),
            self.transitions.clone(),
            self.alphabet_anything_value,
            self.alphabet_symbol_mapping.clone(),
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[pyclass(module = "outlines_core.fsm.outlines_core_rs", name = "Index")]
pub struct PyIndex {
    #[pyo3(get)]
    fsm_info: PyFSMInfo,
    #[pyo3(get)]
    vocabulary: PyVocabulary,
    #[pyo3(get)]
    frozen_tokens: HashSet<String>,
    eos_token_id: u32,
    inner: Option<Index>,
}

#[pymethods]
impl PyIndex {
    #[new]
    fn new(
        fsm_info: PyFSMInfo,
        vocabulary: PyVocabulary,
        eos_token_id: u32,
        frozen_tokens: HashSet<String>,
    ) -> Self {
        Self {
            fsm_info,
            vocabulary,
            eos_token_id,
            frozen_tokens,
            inner: None,
        }
    }

    fn build(&mut self) -> PyResult<()> {
        let fsm_info: FSMInfo = self.fsm_info.clone().into();
        let index = Index::new(
            &fsm_info, &self.vocabulary.0, self.eos_token_id, self.frozen_tokens.clone()
        );
        self.inner = Some(index?);
        Ok(())
    }

    fn get_allowed_tokens(&self, state: u32) -> Option<Vec<u32>> {
        match &self.inner {
            Some(i) => i.allowed_tokens(state),
            None => None,
        }
    }

 // fn get_next_state(&self, state: u32, token_id: u32) -> Option<u32> {
 //     self.0.next_state(state, token_id)
 // }

 // fn is_final_state(&self, state: u32) -> bool {
 //     self.0.is_final(state)
 // }

 // fn get_transitions(&self) -> HashMap<u32, HashMap<u32, u32>> {
 //     self.0.transitions().clone()
 // }

 // fn get_initial_state(&self) -> u32 {
 //     self.0.initial()
 // }

    fn __getstate__(&self, py: Python) -> PyResult<PyObject> {
        let data = serde_json::to_string(&self)
            .map_err(|e| PyException::new_err(format!("Failed to pickle Index: {}", e)))?;
        Ok(PyBytes::new_bound(py, data.as_bytes()).to_object(py))
    }

    fn __setstate__(&mut self, py: Python, state: PyObject) -> PyResult<()> {
        match state.extract::<&[u8]>(py) {
            Ok(s) => {
                *self = serde_json::from_slice(s).map_err(|e| {
                    PyException::new_err(format!("Failed to unpickle Index: {}", e))
                })?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn __getnewargs__(&self) -> PyResult<(PyFSMInfo, PyVocabulary, u32, HashSet<String>)> {
        Ok((
            PyFSMInfo::default(),
            PyVocabulary::default(),
            0,
            HashSet::default(),
        ))
    }
}

#[pyfunction(name = "build_regex_from_schema")]
#[pyo3(signature = (json, whitespace_pattern=None))]
pub fn build_regex_from_schema_py(
    json: String,
    whitespace_pattern: Option<&str>,
) -> PyResult<String> {
    json_schema::build_regex_from_schema(&json, whitespace_pattern)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "to_regex")]
#[pyo3(signature = (json, whitespace_pattern=None))]
pub fn to_regex_py(json: Bound<PyDict>, whitespace_pattern: Option<&str>) -> PyResult<String> {
    let json_value: Value = serde_pyobject::from_pyobject(json)?;
    json_schema::to_regex(&json_value, whitespace_pattern, &json_value)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "_walk_fsm")]
#[pyo3(
    text_signature = "(fsm_transitions, fsm_initial, fsm_finals, token_transition_keys, start_state, full_match)"
)]
pub fn walk_fsm_py(
    fsm_transitions: HashMap<(State, TransitionKey), State>,
    fsm_initial: State,
    fsm_finals: HashSet<State>,
    token_transition_keys: Vec<TransitionKey>,
    start_state: State,
    full_match: bool,
) -> PyResult<Vec<State>> {
    Ok(walk_fsm(
        &fsm_transitions,
        fsm_initial,
        &fsm_finals,
        &token_transition_keys,
        start_state,
        full_match,
    ))
}

#[pyfunction(name = "state_scan_tokens")]
#[pyo3(
    text_signature = "(fsm_transitions, fsm_initial, fsm_finals, vocabulary, vocabulary_transition_keys, start_state)"
)]
pub fn state_scan_tokens_py(
    fsm_transitions: HashMap<(State, TransitionKey), State>,
    fsm_initial: State,
    fsm_finals: HashSet<State>,
    vocabulary: &PyVocabulary,
    vocabulary_transition_keys: HashMap<String, Vec<TransitionKey>>,
    start_state: State,
) -> PyResult<HashSet<(TokenId, State)>> {
    Ok(state_scan_tokens(
        &fsm_transitions,
        fsm_initial,
        &fsm_finals,
        &vocabulary.0,
        &vocabulary_transition_keys,
        start_state,
    ))
}

#[pyfunction(name = "get_token_transition_keys")]
#[pyo3(text_signature = "(alphabet_symbol_mapping, alphabet_anything_value, token_str)")]
pub fn get_token_transition_keys_py(
    alphabet_symbol_mapping: HashMap<String, TransitionKey>,
    alphabet_anything_value: TransitionKey,
    token_str: String,
) -> PyResult<Vec<TransitionKey>> {
    Ok(get_token_transition_keys(
        &alphabet_symbol_mapping,
        alphabet_anything_value,
        &token_str,
    ))
}

#[pyfunction(name = "get_vocabulary_transition_keys")]
#[pyo3(
    text_signature = "(alphabet_symbol_mapping, alphabet_anything_value, vocabulary, frozen_tokens)"
)]
pub fn get_vocabulary_transition_keys_py(
    alphabet_symbol_mapping: HashMap<String, TransitionKey>,
    alphabet_anything_value: TransitionKey,
    vocabulary: &PyVocabulary,
    frozen_tokens: HashSet<String>,
) -> PyResult<HashMap<String, Vec<TransitionKey>>> {
    Ok(get_vocabulary_transition_keys(
        &alphabet_symbol_mapping,
        alphabet_anything_value,
        &vocabulary.0,
        &frozen_tokens,
    ))
}

#[pyfunction(name = "create_fsm_index_end_to_end")]
#[pyo3(text_signature = "(fsm_info, vocabulary, frozen_tokens)")]
pub fn create_fsm_index_end_to_end_py<'py>(
    py: Python<'py>,
    fsm_info: &PyFSMInfo,
    vocabulary: &PyVocabulary,
    frozen_tokens: HashSet<String>,
) -> PyResult<Bound<'py, PyDict>> {
    let states_to_token_subsets = PyDict::new_bound(py);
    let mut seen: HashSet<State> = HashSet::new();
    let mut next_states: HashSet<State> = HashSet::from_iter(vec![fsm_info.initial]);

    let vocabulary_transition_keys = get_vocabulary_transition_keys(
        &fsm_info.alphabet_symbol_mapping,
        fsm_info.alphabet_anything_value,
        &vocabulary.0,
        &frozen_tokens,
    );

    while let Some(start_state) = next_states.iter().cloned().next() {
        next_states.remove(&start_state);

        // TODO: Return Pydict directly at construction
        let token_ids_end_states = state_scan_tokens(
            &fsm_info.transitions,
            fsm_info.initial,
            &fsm_info.finals,
            &vocabulary.0,
            &vocabulary_transition_keys,
            start_state,
        );

        for (token_id, end_state) in token_ids_end_states {
            if let Ok(Some(existing_dict)) = states_to_token_subsets.get_item(start_state) {
                existing_dict.set_item(token_id, end_state)?;
            } else {
                let new_dict = PyDict::new_bound(py);
                new_dict.set_item(token_id, end_state)?;
                states_to_token_subsets.set_item(start_state, new_dict)?;
            }

            if !seen.contains(&end_state) {
                next_states.insert(end_state);
            }
        }

        seen.insert(start_state);
    }

    Ok(states_to_token_subsets)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[pyclass(module = "outlines_core.fsm.outlines_core_rs", name = "Vocabulary")]
pub struct PyVocabulary(Vocabulary);

#[pymethods]
impl PyVocabulary {
    #[staticmethod]
    fn from_dict(map: HashMap<Token, Vec<TokenId>>) -> PyVocabulary {
        PyVocabulary(Vocabulary::from(map))
    }

    #[new]
    #[pyo3(signature = (eos_token_id=None))]
    fn new(eos_token_id: Option<u32>) -> Self {
        PyVocabulary(Vocabulary::new(eos_token_id))
    }

    fn __repr__(&self) -> String {
        format!("{:#?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __getstate__(&self, py: Python) -> PyResult<PyObject> {
        let data = serde_json::to_string(&self)
            .map_err(|e| PyException::new_err(format!("Failed to pickle Vocabulary: {}", e)))?;
        Ok(PyBytes::new_bound(py, data.as_bytes()).to_object(py))
    }

    fn __setstate__(&mut self, py: Python, state: PyObject) -> PyResult<()> {
        match state.extract::<&[u8]>(py) {
            Ok(s) => {
                *self = serde_json::from_slice(s).map_err(|e| {
                    PyException::new_err(format!("Failed to unpickle Vocabulary: {}", e))
                })?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn __getnewargs__(&self) -> PyResult<(Option<u32>,)> {
        Ok((None,))
    }
}

#[pymodule]
fn outlines_core_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(walk_fsm_py, m)?)?;
    m.add_function(wrap_pyfunction!(state_scan_tokens_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_token_transition_keys_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_vocabulary_transition_keys_py, m)?)?;
    m.add_function(wrap_pyfunction!(create_fsm_index_end_to_end_py, m)?)?;

    m.add("BOOLEAN", json_schema::BOOLEAN)?;
    m.add("DATE", json_schema::DATE)?;
    m.add("DATE_TIME", json_schema::DATE_TIME)?;
    m.add("INTEGER", json_schema::INTEGER)?;
    m.add("NULL", json_schema::NULL)?;
    m.add("NUMBER", json_schema::NUMBER)?;
    m.add("STRING", json_schema::STRING)?;
    m.add("STRING_INNER", json_schema::STRING_INNER)?;
    m.add("TIME", json_schema::TIME)?;
    m.add("UUID", json_schema::UUID)?;
    m.add("WHITESPACE", json_schema::WHITESPACE)?;

    m.add_function(wrap_pyfunction!(build_regex_from_schema_py, m)?)?;
    m.add_function(wrap_pyfunction!(to_regex_py, m)?)?;

    m.add_class::<PyIndex>()?;
    m.add_class::<PyVocabulary>()?;
    m.add_class::<PyFSMInfo>()?;

    Ok(())
}
